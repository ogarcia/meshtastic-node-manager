//! Command-line interface.
//!
//! The argument set is declared with clap's derive API; [`command`] then
//! overlays the translated help texts, which have to be set at runtime.

use std::time::Duration;

use clap::{ArgAction, Args, Command, CommandFactory, FromArgMatches, Parser};

use crate::device::{DEFAULT_TCP_PORT, Target};
use crate::fl;
use crate::nodes::{Selection, SortKey};

#[derive(Parser, Debug)]
#[command(name = "meshtastic-node-cleaner", version)]
#[command(disable_help_flag = true, disable_version_flag = true)]
pub struct Cli {
    #[command(flatten)]
    pub connection: Connection,

    #[command(flatten)]
    pub selection: SelectionArgs,

    #[arg(long, value_enum, default_value_t = SortKey::Date)]
    pub sort_by: SortKey,

    #[arg(long)]
    pub reverse: bool,

    #[arg(short = 'n', long, conflicts_with_all = ["yes", "set_time"])]
    pub dry_run: bool,

    #[arg(short, long)]
    pub yes: bool,

    #[arg(long)]
    pub set_time: bool,

    #[arg(short, long, action = ArgAction::Count)]
    pub verbose: u8,

    #[arg(short, long, action = ArgAction::Help)]
    help: Option<bool>,

    #[arg(short = 'V', long, action = ArgAction::Version)]
    version: Option<bool>,
}

#[derive(Args, Debug)]
#[group(multiple = false)]
pub struct Connection {
    #[arg(short, long, visible_alias = "port", value_name = "DEVICE",
          num_args = 0..=1, default_missing_value = "")]
    pub serial: Option<String>,

    #[arg(short, long, visible_alias = "host", value_name = "HOST",
          num_args = 0..=1, default_missing_value = "localhost", value_parser = parse_host)]
    pub tcp: Option<(String, u16)>,
}

#[derive(Args, Debug)]
pub struct SelectionArgs {
    #[arg(short, long)]
    pub unknown: bool,

    #[arg(short, long, value_name = "DURATION", value_parser = parse_duration)]
    pub older_than: Option<Duration>,

    #[arg(long, requires = "older_than")]
    pub include_never: bool,

    #[arg(short, long, value_name = "NODE", num_args = 1.., value_parser = parse_node_id,
          conflicts_with_all = ["unknown", "older_than"])]
    pub remove: Vec<u32>,
}

impl Cli {
    pub fn target(&self) -> Target {
        match (&self.connection.serial, &self.connection.tcp) {
            (_, Some((host, port))) => Target::Tcp(host.clone(), *port),
            (Some(dev), _) if dev.is_empty() => Target::Serial(None),
            (Some(dev), _) => Target::Serial(Some(dev.clone())),
            (None, None) => Target::Auto,
        }
    }

    /// The selection requested on the command line.  With no selector at
    /// all, only partially-known nodes are selected.
    pub fn selection(&self) -> Selection {
        let s = &self.selection;
        if !s.remove.is_empty() {
            return Selection::Explicit(s.remove.clone());
        }
        Selection::Filter {
            unknown: s.unknown || s.older_than.is_none(),
            older_than: s.older_than,
            include_never: s.include_never,
        }
    }
}

/// Parse the process arguments, exiting with a usage error if they are invalid.
pub fn parse() -> Cli {
    let matches = command().get_matches();
    Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit())
}

/// The clap command with translated help texts.
pub fn command() -> Command {
    let conn = fl!("heading-connection");
    let sel = fl!("heading-selection");
    let out = fl!("heading-output");
    let act = fl!("heading-action");

    Cli::command()
        .about(fl!("about"))
        .after_help(fl!("help-examples"))
        .help_template(format!(
            "{{about-with-newline}}\n{} {{usage}}\n\n{{all-args}}{{after-help}}",
            fl!("help-usage")
        ))
        .mut_arg("serial", |a| {
            a.help(fl!("help-serial")).help_heading(conn.clone())
        })
        .mut_arg("tcp", |a| {
            a.help(fl!("help-tcp")).help_heading(conn.clone())
        })
        .mut_arg("unknown", |a| {
            a.help(fl!("help-unknown")).help_heading(sel.clone())
        })
        .mut_arg("older_than", |a| {
            a.help(fl!("help-older-than")).help_heading(sel.clone())
        })
        .mut_arg("include_never", |a| {
            a.help(fl!("help-include-never")).help_heading(sel.clone())
        })
        .mut_arg("remove", |a| {
            a.help(fl!("help-remove")).help_heading(sel.clone())
        })
        .mut_arg("sort_by", |a| {
            a.help(fl!("help-sort-by")).help_heading(out.clone())
        })
        .mut_arg("reverse", |a| {
            a.help(fl!("help-reverse")).help_heading(out.clone())
        })
        .mut_arg("verbose", |a| {
            a.help(fl!("help-verbose")).help_heading(out.clone())
        })
        .mut_arg("dry_run", |a| {
            a.help(fl!("help-dry-run")).help_heading(act.clone())
        })
        .mut_arg("yes", |a| a.help(fl!("help-yes")).help_heading(act.clone()))
        .mut_arg("set_time", |a| {
            a.help(fl!("help-set-time")).help_heading(act.clone())
        })
        .mut_arg("help", |a| {
            a.help(fl!("help-help")).help_heading(out.clone())
        })
        .mut_arg("version", |a| {
            a.help(fl!("help-version")).help_heading(out.clone())
        })
}

/// Parse a duration such as `30m`, `24h`, `7d` or `2w`.
pub fn parse_duration(value: &str) -> Result<Duration, String> {
    let v = value.trim().to_lowercase();
    let invalid = || fl!("error-duration", value = value);
    let (amount, unit) = v
        .split_at_checked(v.len().saturating_sub(1))
        .ok_or_else(invalid)?;
    let secs: u64 = match unit {
        "m" => 60,
        "h" => 3_600,
        "d" => 86_400,
        "w" => 604_800,
        _ => return Err(invalid()),
    };
    if amount.is_empty() || !amount.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    let amount: u64 = amount.parse().map_err(|_| invalid())?;
    amount
        .checked_mul(secs)
        .map(Duration::from_secs)
        .ok_or_else(invalid)
}

/// Parse `host`, `host:port` or `scheme://host[:port]` into host and port.
/// IPv6 addresses need brackets when a port is given (`[fe80::1]:4403`).
pub fn parse_host(value: &str) -> Result<(String, u16), String> {
    let invalid = || fl!("error-host", value = value);
    let raw = value.trim();
    let raw = raw.split_once("://").map_or(raw, |(_, rest)| rest);
    let raw = raw.trim_end_matches('/');
    let (host, port) = match raw.rsplit_once(':') {
        // An IPv6 address without brackets has several colons and no port.
        Some((h, p)) if !h.contains(':') || h.ends_with(']') => {
            (h, p.parse().map_err(|_| invalid())?)
        }
        _ => (raw, DEFAULT_TCP_PORT),
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() || host.contains(['/', ' ']) {
        return Err(invalid());
    }
    Ok((host.to_lowercase(), port))
}

/// Parse a node id given as `!aabbccdd`, `0xaabbccdd` or a decimal number.
pub fn parse_node_id(value: &str) -> Result<u32, String> {
    let v = value.trim();
    let parsed = match v.strip_prefix('!').or_else(|| v.strip_prefix("0x")) {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => v.parse(),
    };
    parsed.map_err(|_| fl!("error-node-id", value = value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;
    use rstest::rstest;

    fn parse_from(args: &[&str]) -> Result<Cli, clap::Error> {
        let argv = std::iter::once("meshtastic-node-cleaner").chain(args.iter().copied());
        let matches = command().try_get_matches_from(argv)?;
        Cli::from_arg_matches(&matches)
    }

    fn err_kind(args: &[&str]) -> ErrorKind {
        parse_from(args).unwrap_err().kind()
    }

    #[test]
    fn command_is_valid() {
        command().debug_assert();
    }

    #[rstest]
    #[case("30m", 30 * 60)]
    #[case("24h", 24 * 3_600)]
    #[case("7d", 7 * 86_400)]
    #[case("2w", 14 * 86_400)]
    #[case(" 7D ", 7 * 86_400)]
    #[case("0d", 0)]
    fn duration_valid(#[case] value: &str, #[case] secs: u64) {
        assert_eq!(parse_duration(value), Ok(Duration::from_secs(secs)));
    }

    #[rstest]
    #[case("")]
    #[case("d")]
    #[case("7")]
    #[case("7x")]
    #[case("-7d")]
    #[case("+7d")]
    #[case("7.5d")]
    #[case("7 d")]
    #[case("1ñ")]
    #[case("99999999999999999999w")]
    fn duration_invalid(#[case] value: &str) {
        assert!(parse_duration(value).is_err());
    }

    #[rstest]
    #[case("Node.local", "node.local", 4403)]
    #[case("192.168.1.10", "192.168.1.10", 4403)]
    #[case("192.168.1.10:5000", "192.168.1.10", 5000)]
    #[case("http://10.0.0.1", "10.0.0.1", 4403)]
    #[case("HTTP://10.0.0.1:1234/", "10.0.0.1", 1234)]
    #[case("[fe80::1]:4000", "fe80::1", 4000)]
    #[case("fe80::1", "fe80::1", 4403)]
    fn host_valid(#[case] value: &str, #[case] host: &str, #[case] port: u16) {
        assert_eq!(parse_host(value), Ok((host.to_string(), port)));
    }

    #[rstest]
    #[case("")]
    #[case("   ")]
    #[case("http://")]
    #[case(":4403")]
    #[case("host:abc")]
    #[case("host:99999")]
    #[case("http://host/path")]
    fn host_invalid(#[case] value: &str) {
        assert!(parse_host(value).is_err());
    }

    #[rstest]
    #[case("!aabbccdd", 0xaabbccdd)]
    #[case("0x10", 16)]
    #[case("42", 42)]
    fn node_id_valid(#[case] value: &str, #[case] num: u32) {
        assert_eq!(parse_node_id(value), Ok(num));
    }

    #[rstest]
    #[case("!zz")]
    #[case("!")]
    #[case("node")]
    #[case("!1aabbccdd")]
    fn node_id_invalid(#[case] value: &str) {
        assert!(parse_node_id(value).is_err());
    }

    #[test]
    fn defaults() {
        let cli = parse_from(&[]).unwrap();
        assert!(matches!(cli.target(), Target::Auto));
        assert_eq!(
            cli.selection(),
            Selection::Filter {
                unknown: true,
                older_than: None,
                include_never: false
            }
        );
        assert_eq!(cli.sort_by, SortKey::Date);
        assert!(!cli.reverse && !cli.dry_run && !cli.yes && !cli.set_time);
        assert_eq!(cli.verbose, 0);
    }

    #[rstest]
    #[case(&["-s"])]
    #[case(&["--serial"])]
    #[case(&["--port"])]
    fn serial_autodetect(#[case] args: &[&str]) {
        assert!(matches!(
            parse_from(args).unwrap().target(),
            Target::Serial(None)
        ));
    }

    #[rstest]
    #[case(&["-s", "/dev/ttyUSB0"])]
    #[case(&["--serial=/dev/ttyUSB0"])]
    #[case(&["--port", "/dev/ttyUSB0"])]
    fn serial_device(#[case] args: &[&str]) {
        let Target::Serial(Some(dev)) = parse_from(args).unwrap().target() else {
            panic!("expected serial target");
        };
        assert_eq!(dev, "/dev/ttyUSB0");
    }

    #[rstest]
    #[case(&["-t"], "localhost", 4403)]
    #[case(&["--tcp"], "localhost", 4403)]
    #[case(&["--host"], "localhost", 4403)]
    #[case(&["-t", "192.168.1.10"], "192.168.1.10", 4403)]
    #[case(&["--tcp", "192.168.1.10:5000"], "192.168.1.10", 5000)]
    #[case(&["--host", "http://node.local:1234"], "node.local", 1234)]
    fn tcp(#[case] args: &[&str], #[case] host: &str, #[case] port: u16) {
        let Target::Tcp(h, p) = parse_from(args).unwrap().target() else {
            panic!("expected TCP target");
        };
        assert_eq!((h.as_str(), p), (host, port));
    }

    #[test]
    fn serial_flag_does_not_swallow_next_flag() {
        let cli = parse_from(&["-s", "-v"]).unwrap();
        assert!(matches!(cli.target(), Target::Serial(None)));
        assert_eq!(cli.verbose, 1);
    }

    #[test]
    fn older_than_alone_selects_by_age_only() {
        let cli = parse_from(&["--older-than", "30d"]).unwrap();
        assert_eq!(
            cli.selection(),
            Selection::Filter {
                unknown: false,
                older_than: Some(Duration::from_secs(30 * 86_400)),
                include_never: false
            }
        );
    }

    #[test]
    fn unknown_and_older_than_combine() {
        let cli = parse_from(&["-u", "-o", "2w", "--include-never"]).unwrap();
        assert_eq!(
            cli.selection(),
            Selection::Filter {
                unknown: true,
                older_than: Some(Duration::from_secs(14 * 86_400)),
                include_never: true
            }
        );
    }

    #[test]
    fn remove_accepts_several_nodes() {
        let cli = parse_from(&["-r", "!0000000a", "11", "--remove", "0xc"]).unwrap();
        assert_eq!(cli.selection(), Selection::Explicit(vec![10, 11, 12]));
    }

    #[rstest]
    #[case(&["-s", "-t"])]
    #[case(&["-r", "!0000000a", "-u"])]
    #[case(&["-r", "!0000000a", "-o", "7d"])]
    #[case(&["-n", "-y"])]
    #[case(&["-n", "--set-time"])]
    fn conflicts(#[case] args: &[&str]) {
        assert_eq!(err_kind(args), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn include_never_requires_older_than() {
        assert_eq!(
            err_kind(&["--include-never"]),
            ErrorKind::MissingRequiredArgument
        );
    }

    #[rstest]
    #[case(&["--older-than", "7x"])]
    #[case(&["-t", "host:abc"])]
    #[case(&["-r", "!zz"])]
    #[case(&["--sort-by", "colour"])]
    fn invalid_values(#[case] args: &[&str]) {
        assert!(matches!(
            err_kind(args),
            ErrorKind::ValueValidation | ErrorKind::InvalidValue
        ));
    }

    #[rstest]
    #[case("date", SortKey::Date)]
    #[case("name", SortKey::Name)]
    #[case("id", SortKey::Id)]
    #[case("hardware", SortKey::Hardware)]
    fn sort_by(#[case] value: &str, #[case] key: SortKey) {
        assert_eq!(parse_from(&["--sort-by", value]).unwrap().sort_by, key);
    }

    #[test]
    fn verbose_count() {
        assert_eq!(parse_from(&["-vv"]).unwrap().verbose, 2);
        assert_eq!(parse_from(&["-v", "-v", "-v"]).unwrap().verbose, 3);
    }
}
