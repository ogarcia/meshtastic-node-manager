//! Command-line interface.
//!
//! The arguments are declared with clap's derive API; [`command`] then
//! overlays the translated help texts, which have to be set at runtime.

use std::time::Duration;

use clap::{ArgAction, Args, Command, CommandFactory, FromArgMatches, Parser, Subcommand};

use crate::device::{DEFAULT_TCP_PORT, Target};
use crate::fl;
use crate::nodes::{Selection, SortKey};

#[derive(Parser, Debug)]
#[command(name = "meshtastic-node-manager", version)]
#[command(
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[arg(short, long, global = true, visible_alias = "port", value_name = "DEVICE",
          num_args = 0..=1, default_missing_value = "", conflicts_with = "tcp")]
    pub serial: Option<String>,

    #[arg(short, long, global = true, visible_alias = "host", value_name = "HOST",
          num_args = 0..=1, default_missing_value = "localhost", value_parser = parse_host)]
    pub tcp: Option<(String, u16)>,

    #[arg(short, long, global = true, action = ArgAction::Count)]
    pub verbose: u8,

    #[arg(short, long, global = true, action = ArgAction::Help)]
    help: Option<bool>,

    #[arg(short = 'V', long, action = ArgAction::Version)]
    version: Option<bool>,

    /// What to do; lists the nodedb when omitted.
    #[command(subcommand)]
    pub command: Option<Cmd>,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
#[command(disable_help_flag = true)]
pub enum Cmd {
    List(ViewArgs),
    Clean(CleanArgs),
    Remove(RemoveArgs),
    Favorite(NodeArgs),
    Unfavorite(NodeArgs),
    Ignore(NodeArgs),
    Unignore(NodeArgs),
    SetTime,
}

impl Default for Cmd {
    fn default() -> Self {
        Cmd::List(ViewArgs::default())
    }
}

/// Table sorting.
#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct ViewArgs {
    #[arg(long, value_enum, default_value_t)]
    pub sort_by: SortKey,

    #[arg(long)]
    pub reverse: bool,
}

/// Confirmation for commands that remove nodes.
#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfirmArgs {
    #[arg(short = 'n', long, conflicts_with = "yes")]
    pub dry_run: bool,

    #[arg(short, long)]
    pub yes: bool,
}

#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct CleanArgs {
    #[arg(short, long)]
    pub unknown: bool,

    #[arg(short, long, value_name = "DURATION", value_parser = parse_duration)]
    pub older_than: Option<Duration>,

    #[arg(long, requires = "older_than")]
    pub include_never: bool,

    #[command(flatten)]
    pub view: ViewArgs,

    #[command(flatten)]
    pub confirm: ConfirmArgs,
}

impl CleanArgs {
    /// With no criterion at all, only partially-known nodes are selected.
    pub fn selection(&self) -> Selection {
        Selection::Filter {
            unknown: self.unknown || self.older_than.is_none(),
            older_than: self.older_than,
            include_never: self.include_never,
        }
    }
}

#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct RemoveArgs {
    #[arg(value_name = "NODE", required = true, num_args = 1.., value_parser = parse_node_id)]
    pub nodes: Vec<u32>,

    #[command(flatten)]
    pub confirm: ConfirmArgs,
}

#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct NodeArgs {
    #[arg(value_name = "NODE", required = true, num_args = 1.., value_parser = parse_node_id)]
    pub nodes: Vec<u32>,
}

impl Cli {
    pub fn target(&self) -> Target {
        match (&self.serial, &self.tcp) {
            (_, Some((host, port))) => Target::Tcp(host.clone(), *port),
            (Some(dev), _) if dev.is_empty() => Target::Serial(None),
            (Some(dev), _) => Target::Serial(Some(dev.clone())),
            (None, None) => Target::Auto,
        }
    }

    pub fn action(&self) -> Cmd {
        self.command.clone().unwrap_or_default()
    }
}

/// Parse the process arguments, exiting with a usage error if they are invalid.
pub fn parse() -> Cli {
    let matches = command().get_matches();
    Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit())
}

/// The clap command with translated help texts.
pub fn command() -> Command {
    localize(
        Cli::command()
            .about(fl!("about"))
            .after_help(fl!("help-examples"))
            .subcommand_help_heading(fl!("heading-commands")),
    )
}

/// Translate the help of `cmd`, its arguments and its subcommands.
fn localize(cmd: Command) -> Command {
    let template = format!(
        "{{about-with-newline}}\n{} {{usage}}\n\n{{all-args}}{{after-help}}",
        fl!("help-usage")
    );
    let mut cmd = cmd.help_template(template);

    let args: Vec<String> = cmd
        .get_arguments()
        .map(|a| a.get_id().to_string())
        .collect();
    for id in args {
        if let Some((help, heading)) = arg_help(&id) {
            cmd = cmd.mut_arg(id, |a| a.help(help).help_heading(heading));
        }
    }

    let subcommands: Vec<String> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    for name in subcommands {
        let about = command_about(&name);
        cmd = cmd.mut_subcommand(name, |s| localize(s.about(about)));
    }
    cmd
}

/// Translated help text and section heading of an argument.
fn arg_help(id: &str) -> Option<(String, String)> {
    let connection = fl!("heading-connection");
    let selection = fl!("heading-selection");
    let output = fl!("heading-output");
    let action = fl!("heading-action");
    let arguments = fl!("heading-arguments");
    Some(match id {
        "serial" => (fl!("help-serial"), connection),
        "tcp" => (fl!("help-tcp"), connection),
        "verbose" => (fl!("help-verbose"), output),
        "help" => (fl!("help-help"), output),
        "version" => (fl!("help-version"), output),
        "sort_by" => (fl!("help-sort-by"), output),
        "reverse" => (fl!("help-reverse"), output),
        "unknown" => (fl!("help-unknown"), selection),
        "older_than" => (fl!("help-older-than"), selection),
        "include_never" => (fl!("help-include-never"), selection),
        "dry_run" => (fl!("help-dry-run"), action),
        "yes" => (fl!("help-yes"), action),
        "nodes" => (fl!("help-nodes"), arguments),
        _ => return None,
    })
}

/// Translated one-line description of a subcommand.
fn command_about(name: &str) -> String {
    match name {
        "list" => fl!("cmd-list"),
        "clean" => fl!("cmd-clean"),
        "remove" => fl!("cmd-remove"),
        "favorite" => fl!("cmd-favorite"),
        "unfavorite" => fl!("cmd-unfavorite"),
        "ignore" => fl!("cmd-ignore"),
        "unignore" => fl!("cmd-unignore"),
        "set-time" => fl!("cmd-set-time"),
        _ => String::new(),
    }
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
        let argv = std::iter::once("meshtastic-node-manager").chain(args.iter().copied());
        let matches = command().try_get_matches_from(argv)?;
        Cli::from_arg_matches(&matches)
    }

    fn cmd(args: &[&str]) -> Cmd {
        parse_from(args).unwrap().action()
    }

    fn err_kind(args: &[&str]) -> ErrorKind {
        parse_from(args).unwrap_err().kind()
    }

    #[test]
    fn command_is_valid() {
        command().debug_assert();
    }

    fn assert_localized(cmd: &Command) {
        for arg in cmd.get_arguments() {
            let id = arg.get_id();
            assert!(
                arg.get_help().is_some(),
                "{}: {id} has no help",
                cmd.get_name()
            );
            assert!(
                arg.get_help_heading().is_some(),
                "{}: {id} has no heading",
                cmd.get_name()
            );
        }
        for sub in cmd.get_subcommands() {
            assert!(
                sub.get_about().is_some_and(|a| !a.to_string().is_empty()),
                "{} has no about",
                sub.get_name()
            );
            assert_localized(sub);
        }
    }

    #[test]
    fn everything_has_translated_help() {
        let mut cmd = command();
        cmd.build();
        assert_localized(&cmd);
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
    fn no_command_lists() {
        let cli = parse_from(&[]).unwrap();
        assert!(matches!(cli.target(), Target::Auto));
        assert_eq!(cli.action(), Cmd::List(ViewArgs::default()));
        assert_eq!(cli.verbose, 0);
    }

    #[rstest]
    #[case(&["-s"])]
    #[case(&["--serial"])]
    #[case(&["--port"])]
    #[case(&["list", "-s"])]
    fn serial_autodetect(#[case] args: &[&str]) {
        assert!(matches!(
            parse_from(args).unwrap().target(),
            Target::Serial(None)
        ));
    }

    #[rstest]
    #[case(&["-s", "/dev/ttyUSB0"])]
    #[case(&["--serial=/dev/ttyUSB0"])]
    #[case(&["--port", "/dev/ttyUSB0", "clean"])]
    #[case(&["clean", "-s", "/dev/ttyUSB0"])]
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
    #[case(&["--tcp", "192.168.1.10:5000", "list"], "192.168.1.10", 5000)]
    #[case(&["set-time", "--host", "http://node.local:1234"], "node.local", 1234)]
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
    fn list_sorting() {
        let Cmd::List(view) = cmd(&["list", "--sort-by", "name", "--reverse"]) else {
            panic!("expected list");
        };
        assert_eq!(view.sort_by, SortKey::Name);
        assert!(view.reverse);
    }

    fn clean(args: &[&str]) -> CleanArgs {
        let argv: Vec<&str> = std::iter::once("clean")
            .chain(args.iter().copied())
            .collect();
        match cmd(&argv) {
            Cmd::Clean(c) => c,
            other => panic!("expected clean, got {other:?}"),
        }
    }

    #[test]
    fn clean_defaults_to_unknown() {
        let c = clean(&[]);
        assert_eq!(
            c.selection(),
            Selection::Filter {
                unknown: true,
                older_than: None,
                include_never: false
            }
        );
        assert_eq!(c.view, ViewArgs::default());
        assert_eq!(c.confirm, ConfirmArgs::default());
    }

    #[test]
    fn clean_older_than_alone_selects_by_age_only() {
        assert_eq!(
            clean(&["--older-than", "30d"]).selection(),
            Selection::Filter {
                unknown: false,
                older_than: Some(Duration::from_secs(30 * 86_400)),
                include_never: false
            }
        );
    }

    #[test]
    fn clean_unknown_and_older_than_combine() {
        let c = clean(&["-u", "-o", "2w", "--include-never", "-n", "--sort-by", "id"]);
        assert_eq!(
            c.selection(),
            Selection::Filter {
                unknown: true,
                older_than: Some(Duration::from_secs(14 * 86_400)),
                include_never: true
            }
        );
        assert!(c.confirm.dry_run);
        assert_eq!(c.view.sort_by, SortKey::Id);
    }

    #[test]
    fn remove_takes_nodes_and_confirmation() {
        let Cmd::Remove(r) = cmd(&["remove", "!0000000a", "11", "0xc", "-y"]) else {
            panic!("expected remove");
        };
        assert_eq!(r.nodes, vec![10, 11, 12]);
        assert!(r.confirm.yes && !r.confirm.dry_run);
    }

    #[rstest]
    #[case("favorite")]
    #[case("unfavorite")]
    #[case("ignore")]
    #[case("unignore")]
    fn flag_commands_take_nodes(#[case] name: &str) {
        let nodes = match cmd(&[name, "!0000000a", "11"]) {
            Cmd::Favorite(n) | Cmd::Unfavorite(n) | Cmd::Ignore(n) | Cmd::Unignore(n) => n.nodes,
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(nodes, vec![10, 11]);
    }

    #[test]
    fn set_time() {
        assert_eq!(cmd(&["set-time"]), Cmd::SetTime);
    }

    #[rstest]
    #[case(&["remove"])]
    #[case(&["favorite"])]
    #[case(&["unignore"])]
    fn node_commands_require_nodes(#[case] args: &[&str]) {
        assert_eq!(err_kind(args), ErrorKind::MissingRequiredArgument);
    }

    #[rstest]
    #[case(&["-s", "-t"])]
    #[case(&["clean", "-n", "-y"])]
    #[case(&["remove", "10", "-n", "-y"])]
    fn conflicts(#[case] args: &[&str]) {
        assert_eq!(err_kind(args), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn include_never_requires_older_than() {
        assert_eq!(
            err_kind(&["clean", "--include-never"]),
            ErrorKind::MissingRequiredArgument
        );
    }

    #[rstest]
    #[case(&["clean", "--older-than", "7x"])]
    #[case(&["-t", "host:abc"])]
    #[case(&["remove", "!zz"])]
    #[case(&["list", "--sort-by", "colour"])]
    fn invalid_values(#[case] args: &[&str]) {
        assert!(matches!(
            err_kind(args),
            ErrorKind::ValueValidation | ErrorKind::InvalidValue
        ));
    }

    #[rstest]
    #[case(&["clean", "-n", "--dry-run"])]
    #[case(&["list", "-o", "7d"])]
    #[case(&["favorite", "10", "-y"])]
    #[case(&["help"])]
    fn rejected(#[case] args: &[&str]) {
        assert!(parse_from(args).is_err());
    }

    #[test]
    fn verbose_count() {
        assert_eq!(parse_from(&["-vv"]).unwrap().verbose, 2);
        assert_eq!(parse_from(&["clean", "-v", "-v", "-v"]).unwrap().verbose, 3);
    }
}
