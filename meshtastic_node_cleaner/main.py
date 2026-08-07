"""Meshtastic Node Cleaner — entry point and orchestration."""

import logging
import sys
import time

from rich.logging import RichHandler
from rich.panel import Panel
from rich.prompt import Confirm

from .cli import build_parser
from .connector import connect
from .display import console, print_banner, print_debug, print_nodes_table, print_removal_results
from .i18n import _
from .nodes import get_all_nodes, get_own_node, human_delta

logger = logging.getLogger(__name__)


def setup_logging(verbose: int) -> None:
    """Configure the root logger based on the -v / -vv verbosity count.

    Levels:
        0 (default) → WARNING  — only real problems surface.
        1 (-v)      → INFO     — connection details and node counts.
        2+ (-vv)    → DEBUG    — per-node processing and full nodedb dump.

    The meshtastic library itself is intentionally kept at WARNING regardless
    of the user's choice, as it is extremely chatty at lower levels.
    """
    level = [logging.WARNING, logging.INFO, logging.DEBUG][min(verbose, 2)]
    logging.basicConfig(
        level=level,
        format="%(message)s",
        datefmt="[%X]",
        handlers=[RichHandler(console=console, show_path=False, markup=True)],
    )
    logging.getLogger("meshtastic").setLevel(logging.WARNING)


def main() -> None:
    """Entry point: parse arguments, connect, display nodes, and delete on confirmation."""
    args = build_parser().parse_args()
    setup_logging(args.verbose)

    print_banner()

    try:
        iface = connect(args)
    except Exception as exc:
        console.print("[bold red]" + _("Connection error:") + f"[/bold red] {exc}")
        sys.exit(1)

    try:
        # The device sends its nodedb during the config handshake right after
        # connection.  A short wait ensures all packets have been received and
        # processed before we query nodesByNum.
        time.sleep(2)
        console.print("[cyan]" + _("Fetching node list…") + "[/cyan]")

        all_nodes = get_all_nodes(iface, args.older_than, args.purge_unknown)
        own_node  = get_own_node(iface)
        to_delete = [n for n in all_nodes if n["delete"]]

        if args.verbose >= 2:
            print_debug(iface, all_nodes)

        logger.info(
            "Node database: %d total node(s), %d scheduled for deletion",
            len(all_nodes), len(to_delete),
        )
        logger.debug(
            "Mode: older_than=%s, purge_unknown=%s | Sort: sort_by=%s, reverse=%s",
            args.older_than, args.purge_unknown, args.sort_by, args.reverse,
        )

        if not all_nodes:
            console.print(Panel(
                "[yellow]" + _("The device has no nodes in its nodedb.") + "[/yellow]",
                border_style="yellow",
            ))
            return

        print_nodes_table(
            all_nodes, args.older_than, args.purge_unknown,
            args.sort_by, args.reverse, own_node,
        )

        if not to_delete:
            if args.purge_unknown:
                msg = (
                    "[bold green]" + _("All clean!") + "[/bold green] " +
                    _("No partially-known nodes found.")
                )
            else:
                msg = (
                    "[bold green]" + _("All clean!") + "[/bold green] " +
                    _(
                        "No nodes inactive for more than [yellow]{age}[/yellow]."
                    ).format(age=human_delta(args.older_than))
                )
            console.print(Panel(msg, border_style="green"))
            return

        console.print(
            "[bold]" +
            _("About to remove [red]{count}[/red] of [white]{total}[/white] node(s).").format(
                count=len(to_delete), total=len(all_nodes)
            ) +
            "[/bold]"
        )
        console.print()

        if args.yes or Confirm.ask(_("Confirm deletion?"), default=False):
            success = 0
            for node in to_delete:
                try:
                    iface.localNode.removeNode(node["num"])
                    console.print(
                        "  [green]✓[/green] " +
                        _("Removed: [cyan]{node_id}[/cyan] ({name})").format(
                            node_id=node["id"], name=node["longName"]
                        )
                    )
                    success += 1
                except Exception as exc:
                    console.print(
                        "  [red]✗[/red] " +
                        _("Error removing {node_id}: {exc}").format(
                            node_id=node["id"], exc=exc
                        )
                    )
            print_removal_results(success, len(to_delete))
        else:
            console.print(
                "[yellow]" +
                _("Operation cancelled. No nodes were removed.") +
                "[/yellow]"
            )

    finally:
        # Always close the interface to release the serial port or TCP socket.
        iface.close()


if __name__ == "__main__":
    main()
