"""Argument parsing and CLI utilities.

Exports:
    parse_duration  — argparse type converter for human duration strings.
    parse_host      — argparse type converter for TCP host/URL strings.
    build_parser    — returns the fully configured ArgumentParser.
"""

import argparse
from datetime import timedelta
from urllib.parse import urlparse

from .connector import DEFAULT_TCP_PORT
from .i18n import _
from .nodes import DEFAULT_MAX_AGE, SORT_KEYS


def parse_duration(value: str) -> timedelta:
    """Convert a human duration string into a timedelta.

    Accepted format: an integer immediately followed by a unit letter.
      m → minutes  |  h → hours  |  d → days  |  w → weeks

    Raises argparse.ArgumentTypeError for any other input.
    """
    units = {"m": "minutes", "h": "hours", "d": "days", "w": "weeks"}
    value = value.strip().lower()
    if len(value) >= 2 and value[-1] in units and value[:-1].isdigit():
        return timedelta(**{units[value[-1]]: int(value[:-1])})
    raise argparse.ArgumentTypeError(
        _("Invalid duration format: '{value}'. Use e.g. 30m, 24h, 7d, 2w").format(
            value=value
        )
    )


def parse_host(value: str) -> tuple[str, int]:
    """Parse a TCP host specification into a (host, port) tuple.

    Accepted formats (all case-insensitive for the scheme):
      hostname              → ('hostname', 4403)
      hostname:port         → ('hostname', port)
      http://hostname       → ('hostname', 4403)
      http://hostname:port  → ('hostname', port)

    The scheme is stripped and ignored; only hostname and optional port are
    extracted.  Any non-empty string that looks like a valid hostname or IP
    address is accepted.

    Args:
        value: Raw string provided by the user on the command line.

    Returns:
        A ``(host, port)`` tuple where host is a non-empty string and port is
        an integer in the valid TCP range.

    Raises:
        argparse.ArgumentTypeError: If the value cannot be parsed or yields
            an empty hostname.
    """
    raw = value.strip()
    if not raw:
        raise argparse.ArgumentTypeError(
            _("Host cannot be empty. Use e.g. 192.168.1.10 or 192.168.1.10:5000")
        )

    # urlparse requires a scheme to correctly split host and port; add a
    # dummy one if the user omitted it (e.g. "192.168.1.10:5000").
    if "://" not in raw:
        raw = "http://" + raw

    try:
        parsed = urlparse(raw)
        host = parsed.hostname  # lowercased, None if absent
        port = parsed.port or DEFAULT_TCP_PORT
    except Exception:
        raise argparse.ArgumentTypeError(
            _("Invalid host: '{value}'").format(value=value)
        )

    if not host:
        raise argparse.ArgumentTypeError(
            _("Invalid host: '{value}'").format(value=value)
        )

    return (host, port)


def build_parser() -> argparse.ArgumentParser:
    """Build and return the argument parser for the CLI.

    Two mutually exclusive groups are defined:

    Connection (pick at most one):
      -s / --serial / --port [DEVICE]  Force serial mode.  If DEVICE is
          omitted, serial ports are auto-detected.  Never falls back to TCP.
      -t / --tcp / --host [HOST]       Force TCP mode.  HOST may be a plain
          hostname, host:port, or http://host:port URL.  Defaults to
          localhost:{DEFAULT_TCP_PORT} when omitted.  Never falls back to serial.
      (neither)                        Auto mode: try serial first, then TCP
                                       on localhost.

    Deletion mode (pick at most one):
      --older-than DURATION  Age-based filter (default).
      --purge-unknown        Remove all partially-known nodes regardless of age.
    """
    parser = argparse.ArgumentParser(
        description=_("Remove nodes from a Meshtastic device's nodedb."),
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=_(
            "Examples:\n"
            "  %(prog)s                           # auto-detect\n"
            "  %(prog)s -s                        # force serial, auto-detect port\n"
            "  %(prog)s -s /dev/ttyUSB0           # force serial, specific device\n"
            "  %(prog)s -t                        # force TCP, localhost\n"
            "  %(prog)s -t 192.168.1.10           # force TCP, specific host\n"
            "  %(prog)s -t 192.168.1.10:5000      # force TCP, host + port\n"
            "  %(prog)s -t 192.168.1.10 --older-than 24h\n"
            "  %(prog)s -s --purge-unknown"
        ),
    )

    # --- connection (mutually exclusive) ---
    conn = parser.add_mutually_exclusive_group()
    conn.add_argument(
        "-s", "--serial", "--port",
        nargs="?",
        const="",       # flag given without a value → auto-detect serial
        default=None,   # flag not given → not in serial mode
        metavar="DEVICE",
        dest="serial",
        help=_(
            "Force serial connection. DEVICE (e.g. /dev/ttyUSB0) is optional;"
            " if omitted, serial ports are auto-detected. Never falls back to TCP"
        ),
    )
    conn.add_argument(
        "-t", "--tcp", "--host",
        nargs="?",
        const=("localhost", DEFAULT_TCP_PORT),  # flag without value → localhost
        default=None,                             # flag not given → not TCP mode
        type=parse_host,
        metavar="HOST",
        dest="host",
        help=_(
            "Force TCP connection. HOST can be a hostname, host:port, or"
            " http://host:port URL. Defaults to localhost if omitted."
            " Never falls back to serial"
        ),
    )

    # --- deletion mode (mutually exclusive) ---
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument(
        "--older-than", type=parse_duration, metavar="DURATION",
        default=DEFAULT_MAX_AGE,
        help=_(
            "Delete all nodes (known and unknown) with no activity for longer"
            " than this duration (e.g. 30m, 24h, 7d, 2w). Default: 7d"
        ),
    )
    mode.add_argument(
        "--purge-unknown", action="store_true",
        help=_(
            "Delete only partially-known nodes (those without complete NodeInfo),"
            " regardless of age"
        ),
    )

    parser.add_argument(
        "--sort-by", choices=list(SORT_KEYS), default="date",
        help=_(
            "Column to sort by. Choices: {choices}. Default: date"
        ).format(choices=", ".join(SORT_KEYS)),
    )
    parser.add_argument(
        "--reverse", action="store_true",
        help=_(
            "Reverse the sort order. For 'date': show oldest first."
            " For 'name'/'id'/'hardware': Z→A order"
        ),
    )
    parser.add_argument(
        "--yes", "-y", action="store_true",
        help=_("Skip confirmation prompt and delete immediately"),
    )
    parser.add_argument(
        "-v", "--verbose", action="count", default=0,
        help=_(
            "Increase verbosity. Use -v for INFO-level output (connection"
            " details, node counts) and -vv for DEBUG-level output (full"
            " nodedb dump, raw processing info)"
        ),
    )

    return parser
