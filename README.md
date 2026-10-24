# Meshtastic Node Cleaner

A command-line tool to remove stale or partially-known nodes from a
[Meshtastic](https://meshtastic.org/) device's nodedb.

Meshtastic radios accumulate nodes over time: devices heard once months ago,
nodes that were never fully identified, or entries left behind by moved
hardware. This tool connects to your node (serial or TCP), shows the nodedb
in a table, and removes the selected nodes after asking for confirmation.

## Features

- **Safe by default**: without options it only selects partially-known nodes
  (those whose NodeInfo was never received).
- **Age-based cleanup** with `--older-than`, optionally combined with
  `--unknown`.
- **Explicit removal** of specific nodes with `--remove`.
- **Dry run** with `--dry-run`: show what would be removed and change nothing.
- **Protected nodes**: your own node, favorites and ignored nodes are never
  removed.
- **Confirmed removals**: each removal waits for the device acknowledgement.
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
./target/release/meshtastic-node-cleaner --help
```

## Usage

```
meshtastic-node-cleaner [OPTIONS]
```

### Connection

| Option | Description |
|--------|-------------|
| `-s`, `--serial`, `--port [DEVICE]` | Connect via serial. `DEVICE` is auto-detected when omitted. Never falls back to TCP. |
| `-t`, `--tcp`, `--host [HOST]` | Connect via TCP. `HOST` may be `host`, `host:port` or `http://host:port`; defaults to `localhost:4403`. Never falls back to serial. |
| *(neither)* | Auto mode: serial first, then TCP on localhost. |

Serial auto-detection only considers USB ports and prefers the USB bridges
used on Meshtastic boards (CP210x, CH340/CH9102, FTDI, Espressif, nRF52,
Seeed, RP2040).

### Selection

| Option | Description |
|--------|-------------|
| `-u`, `--unknown` | Partially-known nodes. **Default** when no other selection is given. |
| `-o`, `--older-than DURATION` | Nodes not heard for longer than `DURATION` (`30m`, `24h`, `7d`, `2w`, …). |
| `--include-never` | With `--older-than`, also select nodes with no last-heard time. |
| `-r`, `--remove NODE...` | Exactly these nodes (`!aabbccdd`, `0xaabbccdd` or decimal). Cannot be combined with the options above. |

`--unknown` and `--older-than` can be combined: both conditions must match.

Favorites and ignored nodes are never removed, even when explicitly
requested. Both flags are stored in the nodedb entry itself, so removing the
node would also drop them.

### Output and action

| Option | Description |
|--------|-------------|
| `--sort-by date\|name\|id\|hardware` | Table sort column. Default: `date` (newest first). |
| `--reverse` | Reverse the sort order. |
| `-n`, `--dry-run` | Show what would be removed without changing anything. |
| `-y`, `--yes` | Remove without asking for confirmation. |
| `--set-time` | Set the device clock to the system time before reading the nodedb. |
| `-v`, `-vv` | Connection details / debug output. |

### Examples

```bash
meshtastic-node-cleaner                    # remove partially-known nodes
meshtastic-node-cleaner -n -o 30d          # preview nodes not heard for 30 days
meshtastic-node-cleaner -t 192.168.1.10    # connect via TCP
meshtastic-node-cleaner -u -o 7d -y        # unknown and older than 7 days, no prompt
meshtastic-node-cleaner -r '!aabbccdd'     # remove one specific node
```

Quote node ids starting with `!`: some shells use it for history expansion.

### Nodes never heard

A node shows *never* in the *Ago* column when the device recorded no
last-heard time for it. This usually means the device had no valid clock
when it heard the node (boards without RTC or GPS only get the time from the
phone app). Such nodes may well be recent, so `--older-than` keeps them
unless `--include-never` is given. `--set-time` gives the device the current
time so that nodes heard from then on get a proper timestamp.

### Reading the table

| Marker | Meaning |
|--------|---------|
| `★` | Your own node (row 0, never removed) |
| `✗` | Selected for removal (listed last) |
| `♥` | Favorite (protected) |
| `⊘` | Ignored (protected) |
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
| `src/main.rs` | Orchestration: connect, show, confirm, remove |
| `src/cli.rs` | Arguments and their parsers |
| `src/device.rs` | Connection, nodedb download and admin requests |
| `src/nodes.rs` | Pure selection and sorting logic |
| `src/display.rs` | Terminal output |
| `src/i18n.rs` | Fluent localisation |

## Translations

Messages live in [Fluent](https://projectfluent.org/) files under
`i18n/<language>/meshtastic_node_cleaner.ftl` and are embedded in the binary
at build time. The language follows `LC_ALL`, `LC_MESSAGES` or `LANG`, with
English as fallback.

English (`i18n/en`) is the reference catalogue: every message used in the
code must exist there, and the build fails otherwise. To add a language,
copy `i18n/en` to `i18n/<code>` and translate the values; messages missing
from a translation fall back to English.
