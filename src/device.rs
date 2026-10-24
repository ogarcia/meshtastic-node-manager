//! Device session: connection, nodedb download and node removal.
//!
//! The `meshtastic` crate only exposes a raw `FromRadio` stream, so the
//! nodedb is rebuilt here from the packets the device sends during the
//! config handshake, and node removal is a hand-built admin packet.

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use log::{debug, info, warn};
use meshtastic::Message;
use meshtastic::api::{ConnectedStreamApi, StreamApi, StreamHandle};
use meshtastic::packet::PacketReceiver;
use meshtastic::protobufs::{self, from_radio, mesh_packet, to_radio};
use meshtastic::utils;
use serialport::SerialPortType;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{Instant, timeout_at};

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
    /// Connect to `target`, run the config handshake and download the nodedb.
    pub async fn connect(target: &Target) -> Result<Self> {
        let (rx, api) = match target {
            Target::Tcp(host, port) => open_tcp(host, *port).await?,
            Target::Serial(Some(dev)) => open_serial(dev).await?,
            Target::Serial(None) => {
                let port = detect_serial_port()?
                    .context("no serial device found; use -t to connect via TCP")?;
                open_serial(&port).await?
            }
            Target::Auto => match detect_serial_port()? {
                Some(port) => open_serial(&port).await?,
                None => {
                    info!("no serial device detected, trying TCP on localhost");
                    open_tcp("localhost", DEFAULT_TCP_PORT).await?
                }
            },
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
                .context("timed out waiting for the device nodedb")?
                .context("connection closed while reading the nodedb")?;

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
            bail!("the device did not report its own node number");
        }
        Ok(())
    }

    /// Ask the device to drop `node_num` from its nodedb.
    ///
    /// Returns `Ok(true)` when the device acknowledged the request and
    /// `Ok(false)` when no acknowledgement arrived in time (the removal may
    /// still have been applied).
    pub async fn remove_node(&mut self, node_num: u32) -> Result<bool> {
        let admin = protobufs::AdminMessage {
            payload_variant: Some(protobufs::admin_message::PayloadVariant::RemoveByNodenum(
                node_num,
            )),
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
        debug!("sent remove_by_nodenum !{node_num:08x} (packet id {packet_id:#x})");

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
            let packet = packet.context("connection closed while waiting for ack")?;

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
                    bail!("device rejected the request: {reason}");
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

async fn open<S>(
    handle: StreamHandle<S>,
) -> (
    PacketReceiver,
    ConnectedStreamApi<meshtastic::api::state::Connected>,
)
where
    S: AsyncReadExt + AsyncWriteExt + Send + 'static,
{
    StreamApi::new().connect(handle).await
}

async fn open_serial(
    device: &str,
) -> Result<(
    PacketReceiver,
    ConnectedStreamApi<meshtastic::api::state::Connected>,
)> {
    info!("connecting via serial to {device}");
    let handle = utils::stream::build_serial_stream(device.to_string(), None, None, None)?;
    Ok(open(handle).await)
}

async fn open_tcp(
    host: &str,
    port: u16,
) -> Result<(
    PacketReceiver,
    ConnectedStreamApi<meshtastic::api::state::Connected>,
)> {
    info!("connecting via TCP to {host}:{port}");
    let handle = utils::stream::build_tcp_stream(format!("{host}:{port}")).await?;
    Ok(open(handle).await)
}

/// Return the first USB serial port, if any.
///
/// Only USB ports are considered, which leaves out the built-in `/dev/ttyS*`
/// UARTs that the generic enumeration also reports.
fn detect_serial_port() -> Result<Option<String>> {
    let mut ports: Vec<_> = serialport::available_ports()?
        .into_iter()
        .filter_map(|p| match p.port_type {
            SerialPortType::UsbPort(usb) => Some((p.port_name, usb)),
            _ => None,
        })
        .collect();
    ports.sort_by(|a, b| a.0.cmp(&b.0));

    for (name, usb) in &ports {
        debug!(
            "serial candidate {name} ({:04x}:{:04x} {})",
            usb.vid,
            usb.pid,
            usb.product.as_deref().unwrap_or("?")
        );
    }
    if ports.len() > 1 {
        warn!(
            "multiple serial ports found, using {}; use -s DEVICE to choose another",
            ports[0].0
        );
    }
    Ok(ports.into_iter().next().map(|(name, _)| name))
}
