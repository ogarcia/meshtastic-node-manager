"""Pure node-data logic — no I/O or display dependencies.

Node entries are represented as plain dicts with the following keys:
  id        (str)           — Meshtastic node ID (e.g. "!aabbccdd")
  num       (int)           — numeric node number as stored in the firmware
  longName  (str)           — human-readable long name
  shortName (str)           — short (4-char) name
  hwModel   (str)           — hardware model string (e.g. "TBEAM")
  lastHeard (datetime|None) — UTC timestamp of last packet received, or None
  ago       (str)           — human-readable elapsed time since lastHeard
  unknown   (bool)          — True if the node has no complete NodeInfo
  delete    (bool)          — True if this node is scheduled for removal
  own       (bool)          — True if this is the local node (never deleted)
"""

import logging
from collections.abc import Callable
from datetime import datetime, timedelta, timezone
from typing import Any

import meshtastic

from .i18n import _

logger = logging.getLogger(__name__)

DEFAULT_MAX_AGE = timedelta(days=7)
"""Default inactivity threshold used when --older-than is not supplied."""

# Sentinel used as a sort key for nodes with no lastHeard timestamp,
# placing them before any real datetime when sorting ascending.
_DATETIME_MIN_UTC: datetime = datetime.min.replace(tzinfo=timezone.utc)

# Sort key functions keyed by the CLI --sort-by choice.
SORT_KEYS: dict[str, Callable[[dict], Any]] = {
    "date":     lambda n: n["lastHeard"] or _DATETIME_MIN_UTC,
    "name":     lambda n: n["longName"].lower(),
    "id":       lambda n: n["id"].lower(),
    "hardware": lambda n: n["hwModel"].lower(),
}

# Fields whose natural sort order is descending (newest / largest first).
# sort_nodes() uses this to decide whether to flip the key before applying --reverse.
SORT_DESC_BY_DEFAULT: set[str] = {"date"}


def _resolve_now(_now: datetime | None) -> datetime:
    """Return *_now* if provided, otherwise the current UTC time.

    Used by public functions that accept ``_now`` for test injection so the
    resolution logic is not repeated in every caller.
    """
    return _now if _now is not None else datetime.now(tz=timezone.utc)


def is_unknown(node: dict) -> bool:
    """Return True if the node was detected but never fully identified.

    The firmware creates a minimal entry when it receives a packet from a node
    whose NodeInfo has not been received yet.  These entries have either no
    'user' field at all, or hwModel set to "UNSET" (the value assigned by
    meshtastic's _getOrCreateByNum helper).
    """
    user = node.get("user")
    if not user:
        return True
    return user.get("hwModel", "UNSET") == "UNSET"


def human_delta(delta: timedelta) -> str:
    """Convert a timedelta to a human-readable string such as '3d 2h 15m'.

    Only non-zero components are included, except when the delta is exactly
    zero — in that case '0m' is returned so the output is never empty.
    """
    total = int(delta.total_seconds())
    days, rem = divmod(total, 86400)
    hours, rem = divmod(rem, 3600)
    minutes = rem // 60
    parts = []
    if days:
        parts.append(f"{days}d")
    if hours:
        parts.append(f"{hours}h")
    if minutes or not parts:  # always emit at least "0m"
        parts.append(f"{minutes}m")
    return " ".join(parts)


def build_node_entry(node_num: int, node: dict, now: datetime) -> dict:
    """Build a normalised node dict from a raw nodedb entry.

    Args:
        node_num: Numeric node identifier (key in iface.nodesByNum).
        node:     Raw node dict as stored by the meshtastic library.
        now:      Reference timestamp used to compute the 'ago' field.

    Returns:
        A dict conforming to the schema described in the module docstring.
        The 'delete' flag is always False here; callers set it afterwards.
    """
    user = node.get("user", {})
    last_heard = node.get("lastHeard")
    lh_dt = datetime.fromtimestamp(last_heard, tz=timezone.utc) if last_heard else None
    delta = (now - lh_dt) if lh_dt else None
    return {
        "id":        user.get("id") or f"!{node_num:08x}",
        "num":       node_num,
        "longName":  user.get("longName", "—"),
        "shortName": user.get("shortName", "—"),
        "hwModel":   user.get("hwModel", "—"),
        "lastHeard": lh_dt,
        "ago":       human_delta(delta) if delta else _("Never seen"),
        "unknown":   is_unknown(node),
        "delete":    False,
        "own":       False,
    }


def get_own_node(
    iface: meshtastic.MeshInterface,
    *,
    _now: datetime | None = None,
) -> dict | None:
    """Return the local node's entry, or None if it cannot be determined.

    The returned dict follows the standard node schema with ``own=True`` and
    ``delete=False`` (own node is never a deletion candidate).

    Args:
        iface: Connected meshtastic interface.
        _now:  Override the current time; intended for unit tests only.

    Returns:
        A node dict for the local node, or None when ``myInfo`` is unavailable
        or the local node number is not found in ``nodesByNum``.
    """
    if not iface.myInfo:
        return None
    my_num = iface.myInfo.my_node_num
    nodes  = iface.nodesByNum or {}
    raw    = nodes.get(my_num)
    if raw is None:
        return None
    now   = _resolve_now(_now)
    entry = build_node_entry(my_num, raw, now)
    entry["own"]    = True
    entry["delete"] = False  # own node is never removed
    return entry


def get_all_nodes(
    iface: meshtastic.MeshInterface,
    max_age: timedelta,
    purge_unknown: bool,
    *,
    _now: datetime | None = None,
) -> list[dict]:
    """Return all nodes on the device (excluding self) with a 'delete' flag.

    Args:
        iface:         Connected meshtastic interface.
        max_age:       Inactivity threshold for the --older-than mode.
        purge_unknown: When True, use --purge-unknown mode (see below).
        _now:          Override the current time; intended for unit tests only.

    Modes:
    - purge_unknown=True  → mark only partially-known nodes for deletion.
    - purge_unknown=False → mark nodes with no activity for longer than
                            max_age (applies equally to known and unknown).

    The returned list is pre-sorted oldest-first so that callers get a stable
    secondary order before applying their own display sort.
    """
    nodes = iface.nodesByNum or {}
    my_id = iface.myInfo.my_node_num if iface.myInfo else None
    now = _resolve_now(_now)
    cutoff = now - max_age
    result = []

    logger.debug(
        "Scanning nodedb: %d entry(ies) in nodesByNum, my_id=%s, cutoff=%s",
        len(nodes), f"!{my_id:08x}" if my_id else None, cutoff,
    )

    for node_num, node in nodes.items():
        if node_num == my_id:
            continue

        entry = build_node_entry(node_num, node, now)

        if purge_unknown:
            entry["delete"] = entry["unknown"]
        else:
            # Nodes never seen are treated as infinitely old.
            entry["delete"] = (
                entry["lastHeard"] is None or entry["lastHeard"] < cutoff
            )

        logger.debug(
            "  !%08x %-20s unknown=%-5s last_heard=%-22s delete=%s",
            node_num,
            entry["longName"][:20],
            entry["unknown"],
            str(entry["lastHeard"]),
            entry["delete"],
        )
        result.append(entry)

    # Pre-sort by lastHeard ascending (oldest first / never-seen at the top).
    # _DATETIME_MIN_UTC acts as a sentinel for nodes with no lastHeard timestamp.
    return sorted(result, key=lambda n: n["lastHeard"] or _DATETIME_MIN_UTC)


def sort_nodes(nodes: list[dict], sort_by: str, reverse: bool) -> list[dict]:
    """Sort a list of node dicts by the given field.

    The reverse flag flips the natural order of the field:
    - For 'date' the natural order is descending (newest first).
    - For all other fields the natural order is ascending (A→Z).
    """
    key = SORT_KEYS[sort_by]
    actual_reverse = (sort_by in SORT_DESC_BY_DEFAULT) ^ reverse
    return sorted(nodes, key=key, reverse=actual_reverse)
