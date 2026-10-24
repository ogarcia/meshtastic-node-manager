//! Device session: connection, nodedb download and node removal.
//!
//! The `meshtastic` crate only exposes a raw `FromRadio` stream, so the
//! nodedb is rebuilt here from the packets the device sends during the
//! config handshake, and admin requests (node removal, clock setting) are
//! hand-built packets.

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use log::{debug, info, warn};
use meshtastic::Message;
use meshtastic::api::{ConnectedStreamApi, StreamApi};
use meshtastic::packet::PacketReceiver;
use meshtastic::protobufs::{self, admin_message, from_radio, mesh_packet, to_radio};
use meshtastic::utils;
use serialport::SerialPortType;
use tokio::time::{Instant, timeout_at};

use crate::fl;

/// Standard Meshtastic TCP port.
pub const DEFAULT_TCP_PORT: u16 = 4403;

/// Maximum time to wait for the device to finish sending its config and
/// nodedb.  Large nodedbs over serial can take a while.
const CONFIG_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum time to wait for the device to acknowledge an admin request.
const ADMIN_ACK_TIMEOUT: Duration = Duration::from_secs(10);

/// Where to connect to.
#[derive(Debug, Clone)]
pub enum Target {
    /// Serial device; `None` means auto-detect.
    Serial(Option<String>),
    /// TCP host and port.
    Tcp(String, u16),
    /// Try serial auto-detection first, then TCP on localhost.
    Auto,
}

/// A concrete device address, once auto-detection has been resolved.
#[derive(Debug, Clone)]
pub enum Endpoint {
    Serial(String),
    Tcp(String, u16),
}

/// An open, configured session with a Meshtastic device.
pub struct Device {
    api: ConnectedStreamApi,
    rx: PacketReceiver,
    /// Node number of the device we are connected to.
    pub my_node_num: u32,
    /// Full nodedb, keyed by node number (includes our own node).
    pub nodes: BTreeMap<u32, protobufs::NodeInfo>,
}

impl Device {
    /// Connect to `endpoint`, run the config handshake and download the nodedb.
    pub async fn connect(endpoint: &Endpoint) -> Result<Self> {
        let (rx, api) = match endpoint {
            Endpoint::Serial(dev) => {
                info!("connecting via serial to {dev}");
                // Assert both DTR and RTS, as pyserial does.  The crate default
                // (DTR on, RTS off) drives GPIO0 low through the usual ESP32
                // auto-reset circuit: that is the user button on many boards,
                // so the device sees a long press, and a reboot in that state
                // lands in the bootloader.
                let handle = utils::stream::build_serial_stream(
                    dev.clone(),
                    None,
                    Some(true),
                    Some(true),
                )?;
                StreamApi::new().connect(handle).await
            }
            Endpoint::Tcp(host, port) => {
                info!("connecting via TCP to {host}:{port}");
                let handle = utils::stream::build_tcp_stream(format!("{host}:{port}")).await?;
                StreamApi::new().connect(handle).await
            }
        };

        let config_id: u32 = utils::generate_rand_id();
        let api = api.configure(config_id).await?;
        let mut device = Device {
            api,
            rx,
            my_node_num: 0,
            nodes: BTreeMap::new(),
        };
        device.read_config(config_id).await?;
        Ok(device)
    }

    /// Consume packets until the device signals the end of the config
    /// handshake, recording our node number and every NodeInfo seen.
    async fn read_config(&mut self, config_id: u32) -> Result<()> {
        let deadline = Instant::now() + CONFIG_TIMEOUT;
        loop {
            let packet = timeout_at(deadline, self.rx.recv())
                .await
                .map_err(|_| anyhow!(fl!("error-config-timeout")))?
                .ok_or_else(|| anyhow!(fl!("error-connection-closed")))?;

            match packet.payload_variant {
                Some(from_radio::PayloadVariant::MyInfo(info)) => {
                    debug!("my_node_num = !{:08x}", info.my_node_num);
                    self.my_node_num = info.my_node_num;
                }
                Some(from_radio::PayloadVariant::NodeInfo(node)) => {
                    debug!("nodeinfo !{:08x}", node.num);
                    self.nodes.insert(node.num, node);
                }
                Some(from_radio::PayloadVariant::ConfigCompleteId(id)) if id == config_id => {
                    info!("config complete: {} node(s) in nodedb", self.nodes.len());
                    break;
                }
                _ => {}
            }
        }
        if self.my_node_num == 0 {
            bail!(fl!("error-no-my-info"));
        }
        Ok(())
    }

    /// Ask the device to drop `node_num` from its nodedb.
    ///
    /// Returns `Ok(true)` when the device acknowledged the request and
    /// `Ok(false)` when no acknowledgement arrived in time (the removal may
    /// still have been applied).
    pub async fn remove_node(&mut self, node_num: u32) -> Result<bool> {
        debug!("removing !{node_num:08x}");
        self.send_admin(admin_message::PayloadVariant::RemoveByNodenum(node_num))
            .await
    }

    /// Set the device clock to `epoch_secs` (Unix time).
    pub async fn set_time(&mut self, epoch_secs: u32) -> Result<bool> {
        debug!("setting device time to {epoch_secs}");
        self.send_admin(admin_message::PayloadVariant::SetTimeOnly(epoch_secs))
            .await
    }

    /// Send an admin request to the connected node and wait for its ack.
    async fn send_admin(&mut self, request: admin_message::PayloadVariant) -> Result<bool> {
        let admin = protobufs::AdminMessage {
            payload_variant: Some(request),
            session_passkey: Vec::new(),
        };
        // Built by hand instead of using send_mesh_packet() so that we know
        // the packet id and can match the routing ack the firmware replies with.
        let packet_id: u32 = utils::generate_rand_id();
        let packet = protobufs::MeshPacket {
            from: self.my_node_num,
            to: self.my_node_num,
            id: packet_id,
            channel: 0,
            payload_variant: Some(mesh_packet::PayloadVariant::Decoded(protobufs::Data {
                portnum: protobufs::PortNum::AdminApp as i32,
                payload: admin.encode_to_vec(),
                want_response: true,
                ..Default::default()
            })),
            ..Default::default()
        };
        self.api
            .send_to_radio_packet(Some(to_radio::PayloadVariant::Packet(packet)))
            .await?;
        debug!("sent admin packet {packet_id:#x}");

        self.wait_for_ack(packet_id).await
    }

    /// Wait for a routing packet answering `request_id`.
    async fn wait_for_ack(&mut self, request_id: u32) -> Result<bool> {
        let deadline = Instant::now() + ADMIN_ACK_TIMEOUT;
        loop {
            let Ok(packet) = timeout_at(deadline, self.rx.recv()).await else {
                warn!("no acknowledgement for request {request_id:#x}");
                return Ok(false);
            };
            let packet = packet.ok_or_else(|| anyhow!(fl!("error-connection-closed")))?;

            let Some(from_radio::PayloadVariant::Packet(mp)) = packet.payload_variant else {
                continue;
            };
            let Some(mesh_packet::PayloadVariant::Decoded(data)) = mp.payload_variant else {
                continue;
            };
            if data.request_id != request_id
                || data.portnum != protobufs::PortNum::RoutingApp as i32
            {
                continue;
            }
            let routing = protobufs::Routing::decode(data.payload.as_slice())?;
            match routing.variant {
                Some(protobufs::routing::Variant::ErrorReason(0)) => return Ok(true),
                Some(protobufs::routing::Variant::ErrorReason(code)) => {
                    let reason = protobufs::routing::Error::try_from(code)
                        .map(|e| e.as_str_name())
                        .unwrap_or("UNKNOWN");
                    bail!(fl!("error-rejected", reason = reason));
                }
                _ => return Ok(true),
            }
        }
    }

    /// Close the connection cleanly.
    pub async fn disconnect(self) -> Result<()> {
        self.api.disconnect().await?;
        Ok(())
    }
}

/// USB vendor ids of the serial bridges and MCUs found on Meshtastic boards.
const KNOWN_VIDS: &[u16] = &[
    0x10c4, // Silicon Labs CP210x
    0x1a86, // WCH CH340 / CH9102
    0x0403, // FTDI
    0x303a, // Espressif native USB
    0x239a, // Adafruit (nRF52 / RAK bootloaders)
    0x2886, // Seeed
    0x2e8a, // Raspberry Pi RP2040
];

/// List USB serial ports, likely Meshtastic devices first.
///
/// Only USB ports are considered, which leaves out the built-in `/dev/ttyS*`
/// UARTs that the generic enumeration also reports.  Ports from vendors not
/// in [`KNOWN_VIDS`] are kept, after the known ones, so unusual boards still
/// work.
pub fn serial_ports() -> Result<Vec<String>> {
    let mut ports: Vec<_> = serialport::available_ports()?
        .into_iter()
        .filter_map(|p| match p.port_type {
            SerialPortType::UsbPort(usb) => Some((p.port_name, usb)),
            _ => None,
        })
        .collect();
    ports.sort_by_key(|(name, usb)| (!KNOWN_VIDS.contains(&usb.vid), name.clone()));

    for (name, usb) in &ports {
        debug!(
            "serial candidate {name} ({:04x}:{:04x} {})",
            usb.vid,
            usb.pid,
            usb.product.as_deref().unwrap_or("?")
        );
    }
    Ok(ports.into_iter().map(|(name, _)| name).collect())
}
