"""Serial and TCP connection logic."""

import argparse
import logging

import meshtastic
import meshtastic.serial_interface
import meshtastic.tcp_interface
import meshtastic.util

from .display import console
from .i18n import _

logger = logging.getLogger(__name__)

DEFAULT_TCP_PORT = 4403
"""Standard Meshtastic TCP port; used as fallback when no port is specified."""


def connect(args: argparse.Namespace) -> meshtastic.MeshInterface:
    """Establish a connection to the Meshtastic node.

    Dispatches to the correct sub-function based on the parsed arguments:

    - ``args.host`` is a ``(host, port)`` tuple  → explicit TCP mode.
    - ``args.serial`` is a string ('' or device path) → explicit serial mode.
    - Both are ``None``                            → auto mode (serial first,
      then TCP on localhost).

    Args:
        args: Parsed CLI arguments from :func:`build_parser`.

    Returns:
        An open :class:`meshtastic.MeshInterface`.

    Raises:
        Any exception raised by the underlying meshtastic interface (e.g.
        ``FileNotFoundError`` for a missing serial device, connection errors
        for TCP).
    """
    if args.host is not None:
        # Explicit TCP mode — never attempts serial.
        host, port = args.host
        return _connect_tcp(host, port)

    if args.serial is not None:
        # Explicit serial mode — never falls back to TCP.
        # args.serial is '' when the flag was given without a device path
        # (auto-detect), or a non-empty string for a specific device.
        return _connect_serial_explicit(args.serial or None)

    # Auto mode: try serial first, then fall back to TCP on localhost.
    return _autoconnect()


# ---------------------------------------------------------------------------
# Private helpers
# ---------------------------------------------------------------------------

def _connect_tcp(host: str, port: int) -> meshtastic.MeshInterface:
    """Open a TCP connection to *host:port*.

    Args:
        host: Hostname or IP address.
        port: TCP port number.

    Returns:
        An open :class:`meshtastic.tcp_interface.TCPInterface`.
    """
    logger.info("Connection method: TCP (%s:%d)", host, port)
    console.print(
        "[cyan]" + _("Connecting via TCP to") + "[/cyan]"
        f" [bold]{host}:{port}[/bold]…"
    )
    return meshtastic.tcp_interface.TCPInterface(hostname=host, portNumber=port)


def _connect_serial_explicit(device: str | None) -> meshtastic.MeshInterface:
    """Open a serial connection to an explicit or auto-detected device.

    This is used when the user explicitly chose serial mode (``-s``).  If no
    serial port is found and ``device`` is ``None``, a :class:`RuntimeError`
    is raised rather than silently falling back to TCP.

    Args:
        device: Device path (e.g. ``/dev/ttyUSB0``) or ``None`` to
                auto-detect.

    Returns:
        An open :class:`meshtastic.serial_interface.SerialInterface`.

    Raises:
        RuntimeError: When auto-detection is requested but no serial port is
            found.
    """
    if device:
        logger.info("Connection method: serial (%s)", device)
        console.print(
            "[cyan]" + _("Connecting via serial to") + "[/cyan]"
            f" [bold]{device}[/bold]…"
        )
        return meshtastic.serial_interface.SerialInterface(devPath=device)

    # Auto-detect within serial mode.
    ports = meshtastic.util.findPorts(eliminate_duplicates=True)
    logger.debug("findPorts() returned: %s", ports)

    if not ports:
        raise RuntimeError(
            _("No serial device found."
              " Connect a device or use -t / --tcp to connect via TCP.")
        )

    return _connect_serial_from_ports(ports)


def _autoconnect() -> meshtastic.MeshInterface:
    """Try serial auto-detection, then fall back to TCP on localhost.

    This is the default behaviour when the user does not specify ``-s`` or
    ``-t``.

    Returns:
        An open :class:`meshtastic.MeshInterface` (either serial or TCP).
    """
    ports = meshtastic.util.findPorts(eliminate_duplicates=True)
    logger.debug("findPorts() returned: %s", ports)

    if ports:
        return _connect_serial_from_ports(ports)

    logger.info("Connection method: TCP fallback (localhost:%d)", DEFAULT_TCP_PORT)
    console.print(
        "[yellow]" +
        _("No serial device detected. Trying TCP on localhost…") +
        "[/yellow]"
    )
    return meshtastic.tcp_interface.TCPInterface(
        hostname="localhost", portNumber=DEFAULT_TCP_PORT
    )


def _connect_serial_from_ports(ports: list[str]) -> meshtastic.MeshInterface:
    """Print the chosen port and open a SerialInterface to the first port.

    Centralises the shared logic used by both :func:`_connect_serial_explicit`
    (auto-detect branch) and :func:`_autoconnect` (serial branch).

    Args:
        ports: Non-empty list of available serial port paths; the first entry
               is used for the connection.

    Returns:
        An open :class:`meshtastic.serial_interface.SerialInterface`.
    """
    _print_serial_port_choice(ports)
    return meshtastic.serial_interface.SerialInterface(devPath=ports[0])


def _print_serial_port_choice(ports: list[str]) -> None:
    """Log and print which serial port will be used.

    If multiple ports are found, the first one is chosen and the user is
    informed that ``--serial`` can be used to select a different one.

    Args:
        ports: Non-empty list of available serial port paths.
    """
    if len(ports) == 1:
        logger.info("Connection method: serial auto-detected (%s)", ports[0])
        console.print(
            "[cyan]" + _("Serial port detected:") + "[/cyan]"
            f" [bold]{ports[0]}[/bold]"
        )
    else:
        logger.info(
            "Connection method: serial auto-detected (%s) — multiple ports: %s",
            ports[0], ports,
        )
        console.print(
            "[yellow]" + _("Multiple serial ports detected:") + "[/yellow]"
            f" {', '.join(ports)}"
        )
        console.print(
            "[dim]" +
            _("Using [bold]{port}[/bold]. Use --serial to specify another.").format(
                port=ports[0]
            ) +
            "[/dim]"
        )
