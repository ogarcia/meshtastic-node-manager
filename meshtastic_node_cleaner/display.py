"""UI layer — all terminal output via rich.

All user-visible strings are wrapped in _() so they can be translated.
The module-level `console` instance is imported by other modules (connector,
main) to guarantee a single shared Console throughout the process.
"""

from datetime import datetime, timedelta, timezone

import meshtastic
from rich import box
from rich.console import Console
from rich.panel import Panel
from rich.table import Table

from .i18n import _
from .nodes import human_delta, sort_nodes

console = Console()


def print_banner() -> None:
    """Print the application title panel."""
    console.print(Panel.fit(
        "[bold cyan]" + _("Meshtastic Node Cleaner") + "[/bold cyan]\n"
        "[dim]" + _("Clean nodes from your device's nodedb") + "[/dim]",
        border_style="cyan",
    ))
    console.print()


def print_nodes_table(
    all_nodes: list[dict],
    max_age: timedelta,
    purge_unknown: bool,
    sort_by: str = "date",
    reverse: bool = False,
    own_node: dict | None = None,
) -> None:
    """Render the full node list as a rich table.

    The local node (if provided) is always shown first as row #0, separated
    from the rest of the list, and is never marked for deletion.
    Nodes scheduled for deletion (delete=True) are marked with ✗ and placed
    at the bottom of the table; nodes to keep (delete=False) come first.
    Within each group the rows are sorted according to sort_by / reverse.

    Args:
        all_nodes:     Complete list of node dicts (excluding own node).
        max_age:       Shown in the table title when purge_unknown=False.
        purge_unknown: Controls which title variant is displayed.
        sort_by:       Column key to sort by (must be a key in SORT_KEYS).
        reverse:       When True, flip the natural sort direction of sort_by.
        own_node:      The local node dict (own=True); shown as row #0 if given.
    """
    to_delete = [n for n in all_nodes if n["delete"]]
    has_unknown = any(n["unknown"] for n in all_nodes)

    if purge_unknown:
        title = _(
            "Device nodes — [bold red]{count}[/bold red] partially-known"
            " node(s) to be deleted [bold red]✗[/bold red]"
        ).format(count=len(to_delete))
    else:
        title = _(
            "Device nodes — [bold red]{count}[/bold red] to be deleted,"
            " inactive for more than"
            " [bold yellow]{age}[/bold yellow] [bold red]✗[/bold red]"
        ).format(count=len(to_delete), age=human_delta(max_age))
        if has_unknown:
            title += "  [dim](" + _("✦ = partially known") + ")[/dim]"

    table = Table(
        title=title,
        box=box.ROUNDED,
        show_lines=True,
        header_style="bold magenta",
    )
    table.add_column(_("#"),        style="dim",   justify="right",  no_wrap=True)
    table.add_column("",                           justify="center", no_wrap=True)
    table.add_column(_("Node ID"),  style="cyan",                    no_wrap=True)
    table.add_column(_("Long name"), style="white")
    table.add_column(_("Short"),    style="yellow", justify="center")
    table.add_column(_("Hardware"), style="blue")
    table.add_column(_("Last seen"), style="green")
    table.add_column(_("Ago"),      style="red")

    # Nodes to keep come first; nodes to delete appear at the bottom.
    # Within each group the chosen sort criterion is applied.
    keep   = sort_nodes([n for n in all_nodes if not n["delete"]], sort_by, reverse)
    delete = sort_nodes([n for n in all_nodes if     n["delete"]], sort_by, reverse)

    def _add_row(row_num: str, node: dict) -> None:
        last = (
            node["lastHeard"].strftime("%Y-%m-%d %H:%M UTC")
            if node["lastHeard"]
            else "—"
        )
        unk_marker = " [bold yellow]✦[/bold yellow]" if node["unknown"] else ""
        if node.get("own"):
            marker = "[bold cyan]★[/bold cyan]"
        elif node["delete"]:
            marker = "[bold red]✗[/bold red]"
        else:
            marker = "[dim]·[/dim]"
        table.add_row(
            row_num,
            marker,
            node["id"] + unk_marker,
            node["longName"],
            node["shortName"],
            node["hwModel"],
            last,
            node["ago"],
        )

    # Row 0: own node (always first, never a deletion candidate).
    if own_node:
        _add_row("0", own_node)

    for i, node in enumerate(keep + delete, 1):
        _add_row(str(i), node)

    console.print()
    console.print(table)
    console.print()


def print_debug(
    iface: meshtastic.MeshInterface,
    all_nodes: list[dict] | None = None,
) -> None:
    """Print a full nodedb diagnostic dump.

    Args:
        iface:     Active Meshtastic interface whose nodesByNum will be dumped.
        all_nodes: Optional list of processed node dicts (as returned by
                   get_all_nodes).  When provided, a ``To delete?`` column is
                   added to the table showing which nodes are deletion candidates.
    """
    nodes_count        = len(iface.nodes)      if iface.nodes      else 0
    nodes_by_num_count = len(iface.nodesByNum) if iface.nodesByNum else 0
    my_id              = iface.myInfo.my_node_num if iface.myInfo  else None

    console.print()
    console.print(Panel(
        "[bold]iface.nodes[/bold]      → "
        f"[yellow]{nodes_count}[/yellow] " + _("entries (nodes with user.id only)") + "\n"
        "[bold]iface.nodesByNum[/bold] → "
        f"[yellow]{nodes_by_num_count}[/yellow] " + _("entries (all nodes)") + "\n"
        "[bold]" + _("My node num") + "[/bold]      → "
        f"[cyan]{my_id}[/cyan]",
        title="[bold magenta]DEBUG — " + _("nodedb summary") + "[/bold magenta]",
        border_style="magenta",
    ))

    if not iface.nodesByNum:
        console.print("[red]" + _("nodesByNum is empty") + "[/red]")
        return

    # Build a lookup from node num → delete flag for O(1) access per row.
    delete_by_num: dict[int, bool] = (
        {n["num"]: n["delete"] for n in all_nodes}
        if all_nodes is not None
        else {}
    )

    table = Table(
        title=_("Full nodesByNum content"),
        box=box.MINIMAL_DOUBLE_HEAD,
        header_style="bold magenta",
        show_lines=False,
    )
    table.add_column(_("num (hex)"),   style="cyan",   no_wrap=True)
    table.add_column("user.id",        style="yellow")
    table.add_column("longName",       style="white")
    table.add_column("hwModel",        style="blue")
    table.add_column("lastHeard",      style="green")
    table.add_column(_("in nodes?"),   justify="center")
    if all_nodes is not None:
        table.add_column(_("to delete?"), justify="center")

    nodes_ids = set(iface.nodes.keys()) if iface.nodes else set()

    for num, node in iface.nodesByNum.items():
        user = node.get("user", {})
        uid  = user.get("id", "—")
        lh   = node.get("lastHeard")
        lh_str = (
            datetime.fromtimestamp(lh, tz=timezone.utc).strftime("%Y-%m-%d %H:%M")
            if lh else "—"
        )
        in_nodes = "[green]✓[/green]" if uid in nodes_ids else "[red]✗[/red]"
        marker   = " [dim](" + _("me") + ")[/dim]" if num == my_id else ""

        row = [
            f"!{num:08x}{marker}",
            uid,
            user.get("longName", "—"),
            user.get("hwModel",  "—"),
            lh_str,
            in_nodes,
        ]
        if all_nodes is not None:
            will_delete = delete_by_num.get(num, False)
            row.append("[bold red]✗[/bold red]" if will_delete else "[dim]·[/dim]")

        table.add_row(*row)

    console.print(table)
    console.print()
    console.print(
        "[dim]" +
        _("Nodes marked ✗ in 'in nodes?' are hidden without --debug") +
        "[/dim]"
    )
    console.print()


def print_removal_results(success: int, total: int) -> None:
    """Print a summary panel after the deletion loop.

    The panel border is green when all removals succeeded, yellow otherwise.
    """
    console.print()
    console.print(Panel(
        _(
            "[bold green]{success}[/bold green] of"
            " [bold]{total}[/bold] node(s) removed."
        ).format(success=success, total=total),
        title=_("Result"),
        border_style="green" if success == total else "yellow",
    ))
