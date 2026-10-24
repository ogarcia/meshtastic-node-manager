//! Meshtastic Node Cleaner — proof of concept.
//!
//! Connects to a device, dumps its nodedb and, only when explicitly asked
//! with `--remove`, deletes a single node.  Read-only by default.

mod device;

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use clap::Parser;
use meshtastic::protobufs::{HardwareModel, NodeInfo};

use device::{DEFAULT_TCP_PORT, Device, Target};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Remove nodes from a Meshtastic device's nodedb (PoC)"
)]
struct Cli {
    /// Force serial connection; DEVICE is auto-detected when omitted
    #[arg(short, long, value_name = "DEVICE", num_args = 0..=1,
          default_missing_value = "", conflicts_with = "tcp")]
    serial: Option<String>,

    /// Force TCP connection; HOST may be host, host:port or http://host:port
    #[arg(short, long, value_name = "HOST", num_args = 0..=1,
          default_missing_value = "localhost", value_parser = parse_host)]
    tcp: Option<(String, u16)>,

    /// Remove this node (e.g. !aabbccdd) after showing the nodedb
    #[arg(long, value_name = "NODE", value_parser = parse_node_id)]
    remove: Option<u32>,

    /// Increase verbosity (-v info, -vv debug)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

/// Parse `host`, `host:port` or `scheme://host[:port]` into host and port.
fn parse_host(value: &str) -> Result<(String, u16), String> {
    let raw = value.trim();
    let raw = raw.split_once("://").map_or(raw, |(_, rest)| rest);
    let raw = raw.trim_end_matches('/');
    let (host, port) = match raw.rsplit_once(':') {
        // An IPv6 address without brackets has several colons and no port.
        Some((h, p)) if !h.contains(':') || h.ends_with(']') => {
            let port = p
                .parse()
                .map_err(|_| format!("invalid port in '{value}'"))?;
            (h, port)
        }
        _ => (raw, DEFAULT_TCP_PORT),
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() {
        return Err(format!("invalid host: '{value}'"));
    }
    Ok((host.to_lowercase(), port))
}

/// Parse a node id given as `!aabbccdd`, `0xaabbccdd` or a decimal number.
fn parse_node_id(value: &str) -> Result<u32, String> {
    let v = value.trim();
    let parsed = if let Some(hex) = v.strip_prefix('!').or_else(|| v.strip_prefix("0x")) {
        u32::from_str_radix(hex, 16)
    } else {
        v.parse()
    };
    parsed.map_err(|_| format!("invalid node id: '{value}'"))
}

fn human_ago(secs: u64) -> String {
    let (d, h, m) = (secs / 86400, secs % 86400 / 3600, secs % 3600 / 60);
    match (d, h) {
        (0, 0) => format!("{m}m"),
        (0, _) => format!("{h}h {m}m"),
        _ => format!("{d}d {h}h"),
    }
}

fn print_nodes(device: &Device) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    let mut nodes: Vec<&NodeInfo> = device.nodes.values().collect();
    // Own node first, then most recently heard first.
    nodes.sort_by_key(|n| (n.num != device.my_node_num, std::cmp::Reverse(n.last_heard)));

    println!(
        "{:<10} {:<4} {:<30} {:<6} {:<20} {:>10}",
        "ID", "FLAG", "LONG NAME", "SHORT", "HARDWARE", "AGO"
    );
    for n in nodes {
        let user = n.user.as_ref();
        let hw = user
            .and_then(|u| HardwareModel::try_from(u.hw_model).ok())
            .map_or("—", |h| h.as_str_name());
        let unknown = user.is_none_or(|u| u.hw_model == HardwareModel::Unset as i32);
        let flags: String = [
            (n.num == device.my_node_num, '★'),
            (n.is_favorite, 'F'),
            (n.is_ignored, 'I'),
            (unknown, '?'),
        ]
        .iter()
        .filter_map(|(on, c)| on.then_some(*c))
        .collect();
        let ago = if n.last_heard == 0 {
            "never".to_string()
        } else {
            human_ago(now.saturating_sub(n.last_heard.into()))
        };
        println!(
            "!{:08x} {:<4} {:<30} {:<6} {:<20} {:>10}",
            n.num,
            flags,
            user.map_or("—", |u| u.long_name.as_str()),
            user.map_or("—", |u| u.short_name.as_str()),
            hw,
            ago,
        );
    }
    println!(
        "\n{} node(s). Flags: ★ own, F favorite, I ignored, ? partially known",
        device.nodes.len()
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let level = ["warn", "info", "debug"][usize::from(cli.verbose.min(2))];
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or(format!("warn,meshtastic_node_cleaner={level}")),
    )
    .init();

    let target = match (&cli.serial, &cli.tcp) {
        (_, Some((host, port))) => Target::Tcp(host.clone(), *port),
        (Some(dev), _) if dev.is_empty() => Target::Serial(None),
        (Some(dev), _) => Target::Serial(Some(dev.clone())),
        (None, None) => Target::Auto,
    };

    let mut device = Device::connect(&target).await.context("connection error")?;
    print_nodes(&device);

    let result = match cli.remove {
        Some(num) if num == device.my_node_num => {
            Err(anyhow::anyhow!("refusing to remove own node"))
        }
        Some(num) if !device.nodes.contains_key(&num) => {
            Err(anyhow::anyhow!("node !{num:08x} is not in the nodedb"))
        }
        Some(num) => match device.remove_node(num).await {
            Ok(true) => {
                println!("Removed !{num:08x} (acknowledged by device)");
                Ok(())
            }
            Ok(false) => {
                println!("Sent removal of !{num:08x}, but the device did not acknowledge it");
                Ok(())
            }
            Err(e) => Err(e),
        },
        None => Ok(()),
    };

    device.disconnect().await?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_variants() {
        let h = |s| parse_host(s).unwrap();
        assert_eq!(h("Node.local"), ("node.local".into(), 4403));
        assert_eq!(h("192.168.1.10:5000"), ("192.168.1.10".into(), 5000));
        assert_eq!(h("http://10.0.0.1"), ("10.0.0.1".into(), 4403));
        assert_eq!(h("http://10.0.0.1:1234/"), ("10.0.0.1".into(), 1234));
        assert_eq!(h("[fe80::1]:4000"), ("fe80::1".into(), 4000));
        assert_eq!(h("fe80::1"), ("fe80::1".into(), 4403));
        assert!(parse_host("").is_err());
        assert!(parse_host("host:abc").is_err());
    }

    #[test]
    fn node_id_variants() {
        assert_eq!(parse_node_id("!aabbccdd"), Ok(0xaabbccdd));
        assert_eq!(parse_node_id("0x10"), Ok(16));
        assert_eq!(parse_node_id("42"), Ok(42));
        assert!(parse_node_id("!zz").is_err());
    }
}
