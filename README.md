# Meshtastic Node Cleaner

A command-line tool to remove stale or partially-known nodes from a
[Meshtastic](https://meshtastic.org/) device's nodedb.

Meshtastic radios accumulate nodes over time — devices heard once months ago,
nodes that were never fully identified, or entries left behind by moved
hardware. This tool connects to your node (via serial or TCP), shows you the
full node list with a clear, colour-coded table, and lets you delete the ones
that no longer belong — after asking for confirmation.

---

## Features

- **Flexible connection** — serial (auto-detect or explicit device path) or TCP
  (hostname / IP, with optional port in URL format). Auto mode tries serial
  first and falls back to TCP on localhost.
- **Age-based cleanup** — delete nodes inactive for longer than a configurable
  duration (`30m`, `24h`, `7d`, `2w`, …). Default: 7 days.
- **Unknown-node purge** — remove only partially-known nodes (those whose
  NodeInfo was never received) regardless of age.
- **Own node always shown** — your local node is pinned at row #0 and is
  never a deletion candidate.
- **Rich terminal UI** — colour-coded table with sort options, deletion
  markers, and a confirmation prompt.
- **Verbosity levels** — `-v` for connection details and counts, `-vv` for a
  full nodedb dump including a *"to delete?"* column.
- **i18n** — all user-visible strings are translatable via gettext `.po`
  files. English and Spanish shipped out of the box.

---

## Requirements

- Python **3.11** or later
- A Meshtastic device connected via USB serial **or** reachable over TCP
- [`uv`](https://github.com/astral-sh/uv) (recommended) **or** standard `pip`

---

## Installation

### With `uv` (recommended)

```bash
git clone https://github.com/youruser/meshtastic-node-cleaner.git
cd meshtastic-node-cleaner
uv sync
scripts/compile-translations.sh
```

This creates a `.venv` virtual environment, installs all dependencies, and
compiles the translation catalogues.

### With `pip`

```bash
git clone https://github.com/youruser/meshtastic-node-cleaner.git
cd meshtastic-node-cleaner
python -m venv .venv
source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install -e .
scripts/compile-translations.sh
```

> **Note:** `.mo` files (compiled gettext catalogues) are not tracked in git.
> You must compile them once after cloning, and again whenever you edit a
> `.po` file. The `scripts/compile-translations.sh` script handles this.
> It requires `msgfmt`, part of the `gettext` package
> (`apt install gettext` / `brew install gettext`).

---

## Running

### Quick launcher (recommended)

A `meshtastic-node-cleaner` script at the repository root activates the
virtual environment automatically:

```bash
./meshtastic-node-cleaner
```

No manual `source .venv/bin/activate` needed.

### From an active virtual environment

```bash
source .venv/bin/activate
meshtastic-node-cleaner          # entry point installed by pyproject.toml
# or
python -m meshtastic_node_cleaner.main
```

---

## Usage

```
meshtastic-node-cleaner [OPTIONS]
```

### Connection options (mutually exclusive)

| Flag | Description |
|------|-------------|
| `-s`, `--serial`, `--port [DEVICE]` | Force serial mode. If `DEVICE` is omitted the tool auto-detects available ports. Never falls back to TCP. |
| `-t`, `--tcp`, `--host [HOST]` | Force TCP mode. `HOST` can be a plain hostname, `host:port`, or `http://host:port` URL. Defaults to `localhost:4403` if omitted. Never falls back to serial. |
| *(neither)* | **Auto mode** — tries serial first, then TCP on localhost. |

### Deletion mode options (mutually exclusive)

| Flag | Description |
|------|-------------|
| `--older-than DURATION` | Delete nodes with no activity for longer than `DURATION`. Accepts `30m`, `24h`, `7d`, `2w`, etc. **Default: `7d`** |
| `--purge-unknown` | Delete only partially-known nodes (no complete NodeInfo), regardless of age. |

### Display options

| Flag | Description |
|------|-------------|
| `--sort-by COLUMN` | Sort the table by `date` (default), `name`, `id`, or `hardware`. |
| `--reverse` | Flip the natural sort direction (oldest-first for `date`; Z→A for text columns). |

### Other options

| Flag | Description |
|------|-------------|
| `-y`, `--yes` | Skip the confirmation prompt and delete immediately. |
| `-v` | INFO verbosity — connection method, node counts. |
| `-vv` | DEBUG verbosity — full nodedb dump with per-node processing details. |
| `-h`, `--help` | Show help and exit. |

### Examples

```bash
# Auto-detect connection, delete nodes inactive for more than 7 days
./meshtastic-node-cleaner

# Force serial, auto-detect port, nodes inactive for more than 24 hours
./meshtastic-node-cleaner -s --older-than 24h

# Connect to a specific serial device
./meshtastic-node-cleaner -s /dev/ttyUSB0

# Connect via TCP to a remote host
./meshtastic-node-cleaner -t 192.168.1.10

# Connect via TCP with a non-standard port
./meshtastic-node-cleaner -t 192.168.1.10:5000

# Remove all partially-known nodes, skip confirmation
./meshtastic-node-cleaner --purge-unknown -y

# Show full nodedb debug dump (includes "to delete?" column)
./meshtastic-node-cleaner -vv

# Sort table by name, oldest nodes first, connect via TCP to localhost
./meshtastic-node-cleaner -t --sort-by name --reverse
```

### Reading the table

```
╭──────────────────────────────────────────────────────╮
│  Device nodes — 2 to be deleted, inactive for …  ✗   │
├───┬───┬────────────┬───────────┬──────┬──────────────┤
│ # │   │ Node ID    │ Long name │ …    │ Ago          │
├───┼───┼────────────┼───────────┼──────┼──────────────┤
│ 0 │ ★ │ !aabbccdd  │ My Node   │ …    │ 5m           │  ← own node (never deleted)
│ 1 │ · │ !11223344  │ Alice     │ …    │ 2h           │  ← keep
│ 2 │ · │ !55667788  │ Bob       │ …    │ 3d           │  ← keep
│ 3 │ ✗ │ !99aabbcc  │ OldNode   │ …    │ 14d          │  ← will be deleted
│ 4 │ ✗ │ !ddeeff00✦ │ —         │ …    │ Never seen   │  ← partially known, will be deleted
╰───┴───┴────────────┴───────────┴──────┴──────────────╯
```

- **★** — your own node (row 0, always kept)
- **·** — node will be kept
- **✗** — node is scheduled for deletion
- **✦** — partially-known node (NodeInfo not received)

Nodes scheduled for deletion always appear at the bottom of the table.

---

## Project structure

```
meshtastic-node-cleaner/
├── meshtastic_node_cleaner/
│   ├── __init__.py
│   ├── main.py          # Entry point & orchestration
│   ├── cli.py           # Argument parsing (parse_duration, parse_host, build_parser)
│   ├── connector.py     # Serial / TCP connection logic
│   ├── nodes.py         # Pure node-data logic (no I/O)
│   ├── display.py       # Rich terminal UI (tables, panels)
│   ├── i18n.py          # gettext setup, exports _()
│   └── locale/
│       ├── en/LC_MESSAGES/meshtastic-node-cleaner.{po,mo}
│       └── es/LC_MESSAGES/meshtastic-node-cleaner.{po,mo}
├── tests/
│   ├── test_cli.py
│   └── test_nodes.py
├── meshtastic-node-cleaner   # Launcher script (runs with .venv)
└── pyproject.toml
```

---

## Development

### Running the tests

```bash
uv run pytest
# or, with the venv active:
pytest
```

### Running a specific test file or class

```bash
pytest tests/test_nodes.py
pytest tests/test_cli.py::TestParseHost
```

### Code style

- Python 3.11+ type annotations throughout.
- All user-visible strings wrapped in `_()` for i18n.
- Logging messages are **not** translated (developer-facing).
- `nodes.py` has no I/O dependencies — keep it that way for testability.

---

## Contributing

1. Fork the repository and create a feature branch.
2. Make your changes. Keep `nodes.py` free of I/O. Keep display logic in
   `display.py`.
3. Add or update tests in `tests/`. Run the full suite before opening a PR:
   ```bash
   uv run pytest
   ```
4. If you add user-visible strings, wrap them in `_()` and update the `.po`
   files (see [Translations](#translations) below).
5. Open a pull request describing what you changed and why.

---

## Translations

All user-visible strings live in `.po` files under
`meshtastic_node_cleaner/locale/`. The `.mo` files are compiled binary
catalogues — they are **not tracked in git** and must be generated locally.
Never edit `.mo` files by hand; always regenerate them from the `.po` source.

### Compiling catalogues

After cloning, or after editing any `.po` file, run:

```bash
scripts/compile-translations.sh
```

This compiles all languages at once. It requires `msgfmt` from the `gettext`
package (`apt install gettext` / `brew install gettext`).

To compile a single language manually:

```bash
msgfmt meshtastic_node_cleaner/locale/es/LC_MESSAGES/meshtastic-node-cleaner.po \
       -o meshtastic_node_cleaner/locale/es/LC_MESSAGES/meshtastic-node-cleaner.mo
```

### Updating an existing translation

1. Edit the relevant `.po` file, e.g. for Spanish:

   ```
   meshtastic_node_cleaner/locale/es/LC_MESSAGES/meshtastic-node-cleaner.po
   ```

   Each entry looks like:

   ```po
   msgid "Confirm deletion?"
   msgstr "¿Confirmar eliminación?"
   ```

   Change only the `msgstr` line. Never change `msgid`.

2. After editing, recompile the catalogues:

   ```bash
   scripts/compile-translations.sh
   ```

3. Test your translation by setting the locale before running:

   ```bash
   LANG=es_ES.UTF-8 ./meshtastic-node-cleaner --help
   ```

### Adding a new language

1. Create the directory structure for your locale (replace `fr` with your
   language code):

   ```bash
   mkdir -p meshtastic_node_cleaner/locale/fr/LC_MESSAGES
   ```

2. Copy the English `.po` file as a starting point:

   ```bash
   cp meshtastic_node_cleaner/locale/en/LC_MESSAGES/meshtastic-node-cleaner.po \
      meshtastic_node_cleaner/locale/fr/LC_MESSAGES/meshtastic-node-cleaner.po
   ```

3. Edit the new file: update the `Language:` header and translate every
   `msgstr` line. Leave `msgstr` identical to `msgid` for any string you
   haven't translated yet — it will fall back to the source string:

   ```po
   msgstr ""
   "Content-Type: text/plain; charset=UTF-8\n"
   "Content-Transfer-Encoding: 8bit\n"
   "Language: fr\n"

   msgid "Confirm deletion?"
   msgstr "Confirmer la suppression ?"
   ```

   > **Important:** Rich markup tags such as `[bold red]`, `[/bold red]`, and
   > format placeholders like `{count}` or `{age}` must be preserved exactly
   > as they appear in the `msgid`.

4. Compile the catalogues:

   ```bash
   scripts/compile-translations.sh
   ```

5. Test it:

   ```bash
   LANG=fr_FR.UTF-8 ./meshtastic-node-cleaner
   ```

6. Add the new `.po` file to your pull request.

### When new strings are added to the code

When a developer adds a new `_("…")` string to the source, all `.po` files
need a new entry. Add the following block at the end of each `.po` file:

```po
msgid "The new string exactly as it appears in the source"
msgstr ""
```

Leave `msgstr` empty for languages not yet translated. Then recompile the
`.mo` files for all languages.

---

## License

[GNU General Public License v3.0](LICENSE)
