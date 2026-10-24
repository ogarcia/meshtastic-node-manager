# Meshtastic Node Manager

A command-line tool to inspect and clean the nodedb of a
[Meshtastic](https://meshtastic.org/) device.

Meshtastic radios accumulate nodes over time: devices heard once months ago,
nodes that were never fully identified, or entries left behind by moved
hardware. This tool connects to your node (serial or TCP), shows the nodedb
in a table, removes stale or partially-known nodes after asking for
confirmation, and manages favorites and ignored nodes.

## Features

- **Read-only by default**: without a command it just lists the nodedb.
- **Cleanup** of partially-known nodes (those whose NodeInfo was never
  received) or of nodes not heard for a given time, with a dry-run mode.
- **Explicit removal** of specific nodes.
- **Favorites and ignored nodes**: mark and unmark them. They are never
  removed, and neither is your own node.
- **Confirmed changes**: each request waits for the device acknowledgement.
- **Flexible connection**: serial (auto-detected or explicit) or TCP. Auto
  mode tries serial first and falls back to TCP on localhost.
- **Translated** into English and Spanish, following the system locale.
- **Single static binary**: pure Rust, no runtime dependencies.

## Installation

You need a recent stable Rust toolchain (edition 2024).

```bash
cargo install --path .
```

Or build it and run it from the repository:

```bash
cargo build --release
./target/release/meshtastic-node-manager --help
```

## Usage

```
meshtastic-node-manager [OPTIONS] [COMMAND]
```

| Command | Description |
|---------|-------------|
| `list` | Show the nodedb. **Default** when no command is given. |
| `clean` | Remove partially-known or stale nodes. |
| `remove NODE...` | Remove specific nodes. |
| `favorite NODE...` / `unfavorite NODE...` | Mark or unmark nodes as favorites. |
| `ignore NODE...` / `unignore NODE...` | Ignore nodes or stop ignoring them. |
| `set-time` | Set the device clock to the system time. |

`NODE` is a node id: `!aabbccdd`, `0xaabbccdd` or a decimal number. Quote ids
starting with `!`, as some shells use it for history expansion.

Run `meshtastic-node-manager <COMMAND> --help` for the options of each
command.

### Connection

These options can go before or after the command.

| Option | Description |
|--------|-------------|
| `-s`, `--serial`, `--port [DEVICE]` | Connect via serial. `DEVICE` is auto-detected when omitted. Never falls back to TCP. |
| `-t`, `--tcp`, `--host [HOST]` | Connect via TCP. `HOST` may be `host`, `host:port` or `http://host:port`; defaults to `localhost:4403`. Never falls back to serial. |
| *(neither)* | Auto mode: serial first, then TCP on localhost. |
| `-v`, `-vv` | Connection details / debug output. |

Serial auto-detection only considers USB ports and prefers the USB bridges
used on Meshtastic boards (CP210x, CH340/CH9102, FTDI, Espressif, nRF52,
Seeed, RP2040).

### Cleaning

| Option | Description |
|--------|-------------|
| `-u`, `--unknown` | Partially-known nodes. **Default** when no other criterion is given. |
| `-o`, `--older-than DURATION` | Nodes not heard for longer than `DURATION` (`30m`, `24h`, `7d`, `2w`, …). |
| `--include-never` | With `--older-than`, also select nodes with no last-heard time. |
| `-n`, `--dry-run` | Show what would be removed without changing anything. |
| `-y`, `--yes` | Remove without asking for confirmation. |
| `--sort-by date\|name\|id\|hardware` | Table sort column. Default: `date` (newest first). Also for `list`. |
| `--reverse` | Reverse the sort order. Also for `list`. |

`--unknown` and `--older-than` can be combined: both conditions must match.
`remove` also accepts `--dry-run` and `--yes`.

### Protected nodes

Your own node, favorites and ignored nodes are never removed, not even with
`remove`. The favorite and ignore flags are stored in the nodedb entry
itself, so removing the node would also drop them. To remove one of these
nodes, unmark it first:

```bash
meshtastic-node-manager unfavorite '!aabbccdd'
meshtastic-node-manager remove '!aabbccdd'
```

### Examples

```bash
meshtastic-node-manager                        # show the nodedb
meshtastic-node-manager -t 192.168.1.10 list   # same, via TCP
meshtastic-node-manager clean -n               # preview partially-known nodes
meshtastic-node-manager clean -o 30d           # nodes not heard for 30 days
meshtastic-node-manager clean -u -o 7d -y      # unknown and older than 7 days, no prompt
meshtastic-node-manager remove '!aabbccdd'     # one specific node
meshtastic-node-manager favorite '!aabbccdd'   # mark a favorite
meshtastic-node-manager ignore '!aabbccdd'     # ignore a node
```

### Nodes never heard

A node shows *never* in the *Ago* column when the device recorded no
last-heard time for it. This usually means the device had no valid clock
when it heard the node (boards without RTC or GPS only get the time from the
phone app). Such nodes may well be recent, so `clean --older-than` keeps them
unless `--include-never` is given. `set-time` gives the device the current
time so that nodes heard from then on get a proper timestamp.

### Reading the table

| Marker | Meaning |
|--------|---------|
| `★` | Your own node (row 0, never removed) |
| `♥` | Favorite (listed after your own node, protected) |
| `⊘` | Ignored (protected) |
| `✗` | Selected for removal (listed last) |
| `✦` | Next to the id: partially-known node |

## Development

```bash
cargo test
cargo clippy --all-targets
cargo fmt
```

The code is split into:

| Module | Purpose |
|--------|---------|
| `src/main.rs` | Orchestration: one function per command |
| `src/cli.rs` | Commands, arguments and their parsers |
| `src/device.rs` | Connection, nodedb download and admin requests |
| `src/nodes.rs` | Pure selection and sorting logic |
| `src/display.rs` | Terminal output |
| `src/i18n.rs` | Fluent localisation |

## Translations

Messages live in [Fluent](https://projectfluent.org/) files under
`i18n/<language>/meshtastic_node_manager.ftl` and are embedded in the binary
at build time. The language follows `LC_ALL`, `LC_MESSAGES` or `LANG`, with
English as fallback.

English (`i18n/en`) is the reference catalogue: every message used in the
code must exist there, and the build fails otherwise. To add a language,
copy `i18n/en` to `i18n/<code>` and translate the values; messages missing
from a translation fall back to English.
