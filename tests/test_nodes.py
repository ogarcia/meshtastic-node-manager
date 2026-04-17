"""Tests for meshtastic_node_cleaner.nodes."""

from datetime import datetime, timedelta, timezone
from unittest.mock import MagicMock

import pytest

from meshtastic_node_cleaner.nodes import (
    DEFAULT_MAX_AGE,
    build_node_entry,
    get_all_nodes,
    get_own_node,
    human_delta,
    is_unknown,
    sort_nodes,
)

# Fixed reference timestamp used across all tests to keep time-based
# assertions fully deterministic regardless of when the suite is run.
NOW = datetime(2024, 6, 1, 12, 0, 0, tzinfo=timezone.utc)


# ---------------------------------------------------------------------------
# is_unknown
# ---------------------------------------------------------------------------

class TestIsUnknown:
    def test_no_user_field(self):
        assert is_unknown({}) is True

    def test_user_is_none(self):
        assert is_unknown({"user": None}) is True

    def test_hwmodel_unset(self):
        assert is_unknown({"user": {"hwModel": "UNSET"}}) is True

    def test_hwmodel_missing_defaults_to_unset(self):
        assert is_unknown({"user": {"longName": "Test"}}) is True

    def test_known_node(self):
        assert is_unknown({"user": {"hwModel": "TLORA_V2"}}) is False

    def test_known_node_tbeam(self):
        assert is_unknown({"user": {"hwModel": "TBEAM"}}) is False


# ---------------------------------------------------------------------------
# human_delta
# ---------------------------------------------------------------------------

class TestHumanDelta:
    def test_minutes_only(self):
        assert human_delta(timedelta(minutes=45)) == "45m"

    def test_hours_and_minutes(self):
        assert human_delta(timedelta(hours=2, minutes=30)) == "2h 30m"

    def test_days_hours_minutes(self):
        assert human_delta(timedelta(days=3, hours=1, minutes=5)) == "3d 1h 5m"

    def test_zero(self):
        assert human_delta(timedelta(0)) == "0m"

    def test_exact_day(self):
        assert human_delta(timedelta(days=1)) == "1d"

    def test_exact_hour(self):
        assert human_delta(timedelta(hours=1)) == "1h"

    def test_weeks(self):
        assert human_delta(timedelta(weeks=2)) == "14d"


# ---------------------------------------------------------------------------
# build_node_entry
# ---------------------------------------------------------------------------

class TestBuildNodeEntry:
    def _raw(self, uid="!aabbccdd", long="Test Node", short="TN",
             hw="TBEAM", last_heard=None):
        node = {"user": {"id": uid, "longName": long, "shortName": short, "hwModel": hw}}
        if last_heard is not None:
            node["lastHeard"] = int(last_heard.timestamp())
        return node

    def test_basic_fields(self):
        raw = self._raw()
        entry = build_node_entry(0xAABBCCDD, raw, NOW)
        assert entry["id"] == "!aabbccdd"
        assert entry["num"] == 0xAABBCCDD
        assert entry["longName"] == "Test Node"
        assert entry["shortName"] == "TN"
        assert entry["hwModel"] == "TBEAM"
        assert entry["delete"] is False
        assert entry["unknown"] is False
        assert entry["own"] is False

    def test_no_last_heard(self):
        raw = self._raw()
        entry = build_node_entry(1, raw, NOW)
        assert entry["lastHeard"] is None
        assert entry["ago"] == "Never seen"

    def test_with_last_heard(self):
        lh = NOW - timedelta(hours=3)
        raw = self._raw(last_heard=lh)
        entry = build_node_entry(1, raw, NOW)
        assert entry["lastHeard"] == lh
        assert entry["ago"] == "3h"

    def test_fallback_id_when_no_user(self):
        entry = build_node_entry(0x0000ABCD, {}, NOW)
        assert entry["id"] == "!0000abcd"
        assert entry["unknown"] is True

    def test_unknown_node(self):
        raw = {"user": {"hwModel": "UNSET", "longName": "Meshtastic abcd", "shortName": "abcd"}}
        entry = build_node_entry(1, raw, NOW)
        assert entry["unknown"] is True


# ---------------------------------------------------------------------------
# sort_nodes
# ---------------------------------------------------------------------------

def _make_node(node_id, long_name, hw, last_heard_offset_days):
    """Return a minimal node dict for sort_nodes tests."""
    lh = NOW - timedelta(days=last_heard_offset_days) if last_heard_offset_days is not None else None
    return {
        "id": node_id,
        "longName": long_name,
        "hwModel": hw,
        "lastHeard": lh,
        "delete": False,
        "unknown": False,
        "own": False,
        "num": 0,
        "shortName": "",
        "ago": "",
    }


# Sample with three nodes spanning different dates, names, and hardware models.
NODES_SAMPLE = [
    _make_node("!c", "Zebra",  "TBEAM",  10),
    _make_node("!a", "Alpha",  "TLORA",   1),
    _make_node("!b", "Mango",  "TBEAM",   5),
]


class TestSortNodes:
    def test_date_default_newest_first(self):
        result = sort_nodes(NODES_SAMPLE, "date", reverse=False)
        assert [n["id"] for n in result] == ["!a", "!b", "!c"]

    def test_date_reverse_oldest_first(self):
        result = sort_nodes(NODES_SAMPLE, "date", reverse=True)
        assert [n["id"] for n in result] == ["!c", "!b", "!a"]

    def test_name_az(self):
        result = sort_nodes(NODES_SAMPLE, "name", reverse=False)
        assert [n["longName"] for n in result] == ["Alpha", "Mango", "Zebra"]

    def test_name_za(self):
        result = sort_nodes(NODES_SAMPLE, "name", reverse=True)
        assert [n["longName"] for n in result] == ["Zebra", "Mango", "Alpha"]

    def test_id_az(self):
        result = sort_nodes(NODES_SAMPLE, "id", reverse=False)
        assert [n["id"] for n in result] == ["!a", "!b", "!c"]

    def test_hardware_az(self):
        result = sort_nodes(NODES_SAMPLE, "hardware", reverse=False)
        assert result[0]["hwModel"] <= result[-1]["hwModel"]


# ---------------------------------------------------------------------------
# get_own_node
# ---------------------------------------------------------------------------

class TestGetOwnNode:
    def _node(self, uid="!aabbccdd", hw="TBEAM", last_heard_offset_days=1):
        n = {"user": {"id": uid, "longName": "Me", "shortName": "ME", "hwModel": hw}}
        if last_heard_offset_days is not None:
            n["lastHeard"] = int((NOW - timedelta(days=last_heard_offset_days)).timestamp())
        return n

    def test_returns_own_node(self):
        iface = _make_iface({1: self._node()}, my_node_num=1)
        result = get_own_node(iface, _now=NOW)
        assert result is not None
        assert result["own"] is True
        assert result["delete"] is False

    def test_own_node_id(self):
        iface = _make_iface({1: self._node(uid="!aabbccdd")}, my_node_num=1)
        result = get_own_node(iface, _now=NOW)
        assert result["id"] == "!aabbccdd"

    def test_returns_none_when_no_my_info(self):
        iface = _make_iface({1: self._node()}, my_node_num=1)
        iface.myInfo = None
        assert get_own_node(iface, _now=NOW) is None

    def test_returns_none_when_num_not_in_db(self):
        """Own node number not present in nodesByNum."""
        iface = _make_iface({2: self._node()}, my_node_num=1)
        assert get_own_node(iface, _now=NOW) is None

    def test_own_node_not_in_get_all_nodes(self):
        """get_all_nodes must still exclude own node from its list."""
        iface = _make_iface(
            {1: self._node("!own"), 2: self._node("!other")},
            my_node_num=1,
        )
        result = get_all_nodes(iface, DEFAULT_MAX_AGE, purge_unknown=False, _now=NOW)
        assert all(n["id"] != "!own" for n in result)


# ---------------------------------------------------------------------------
# get_all_nodes
# ---------------------------------------------------------------------------

def _make_iface(nodes_by_num: dict, my_node_num: int = 0):
    """Return a MagicMock that mimics a meshtastic.MeshInterface."""
    iface = MagicMock()
    iface.nodesByNum = nodes_by_num
    iface.myInfo.my_node_num = my_node_num
    return iface


class TestGetAllNodes:
    def _node(self, uid, hw="TBEAM", last_heard_offset_days=None):
        n = {"user": {"id": uid, "longName": uid, "shortName": uid[:2], "hwModel": hw}}
        if last_heard_offset_days is not None:
            n["lastHeard"] = int((NOW - timedelta(days=last_heard_offset_days)).timestamp())
        return n

    def test_excludes_own_node(self):
        nodes = {1: self._node("!own", last_heard_offset_days=0), 2: self._node("!other", last_heard_offset_days=1)}
        iface = _make_iface(nodes, my_node_num=1)
        result = get_all_nodes(iface, DEFAULT_MAX_AGE, purge_unknown=False, _now=NOW)
        assert len(result) == 1
        assert result[0]["id"] == "!other"

    def test_older_than_marks_delete(self):
        nodes = {
            1: self._node("!new", last_heard_offset_days=1),
            2: self._node("!old", last_heard_offset_days=30),
        }
        iface = _make_iface(nodes, my_node_num=0)
        result = get_all_nodes(iface, timedelta(days=7), purge_unknown=False, _now=NOW)
        by_id = {n["id"]: n for n in result}
        assert by_id["!new"]["delete"] is False
        assert by_id["!old"]["delete"] is True

    def test_never_seen_marked_delete(self):
        nodes = {1: self._node("!ghost")}  # no lastHeard field
        iface = _make_iface(nodes, my_node_num=0)
        result = get_all_nodes(iface, timedelta(days=7), purge_unknown=False, _now=NOW)
        assert result[0]["delete"] is True

    def test_purge_unknown_only_marks_unknown(self):
        nodes = {
            1: self._node("!known", hw="TBEAM", last_heard_offset_days=30),
            2: self._node("!unk",   hw="UNSET", last_heard_offset_days=1),
        }
        iface = _make_iface(nodes, my_node_num=0)
        result = get_all_nodes(iface, timedelta(days=7), purge_unknown=True, _now=NOW)
        by_id = {n["id"]: n for n in result}
        assert by_id["!known"]["delete"] is False
        assert by_id["!unk"]["delete"] is True

    def test_empty_nodedb(self):
        iface = _make_iface({}, my_node_num=0)
        assert get_all_nodes(iface, DEFAULT_MAX_AGE, purge_unknown=False, _now=NOW) == []
