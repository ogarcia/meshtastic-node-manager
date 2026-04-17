"""Tests for meshtastic_node_cleaner.cli (parse_duration, parse_host, build_parser)."""

import argparse
from datetime import timedelta

import pytest

from meshtastic_node_cleaner.cli import build_parser, parse_duration, parse_host
from meshtastic_node_cleaner.nodes import DEFAULT_MAX_AGE


# ---------------------------------------------------------------------------
# parse_duration
# ---------------------------------------------------------------------------

class TestParseDuration:
    @pytest.mark.parametrize("value,expected", [
        ("30m",  timedelta(minutes=30)),
        ("1h",   timedelta(hours=1)),
        ("7d",   timedelta(days=7)),
        ("2w",   timedelta(weeks=2)),
        ("  7D ", timedelta(days=7)),   # whitespace + uppercase
        ("0m",   timedelta(0)),
    ])
    def test_valid(self, value, expected):
        assert parse_duration(value) == expected

    @pytest.mark.parametrize("value", [
        "7",       # no unit
        "d",       # no number
        "1.5h",    # decimal
        "abc",     # invalid
        "",        # empty
        "7x",      # unknown unit
    ])
    def test_invalid_raises(self, value):
        with pytest.raises(argparse.ArgumentTypeError):
            parse_duration(value)


# ---------------------------------------------------------------------------
# parse_host
# ---------------------------------------------------------------------------

class TestParseHost:
    @pytest.mark.parametrize("value,expected_host,expected_port", [
        ("192.168.1.10",            "192.168.1.10",  4403),
        ("192.168.1.10:5000",       "192.168.1.10",  5000),
        ("http://192.168.1.10",     "192.168.1.10",  4403),
        ("http://192.168.1.10:5000","192.168.1.10",  5000),
        ("meshtastic.local",        "meshtastic.local", 4403),
        ("meshtastic.local:9000",   "meshtastic.local", 9000),
        ("localhost",               "localhost",      4403),
        ("localhost:4403",          "localhost",      4403),
        # hostname is lowercased by urlparse
        ("MyNode.local",            "mynode.local",   4403),
    ])
    def test_valid(self, value, expected_host, expected_port):
        host, port = parse_host(value)
        assert host == expected_host
        assert port == expected_port

    @pytest.mark.parametrize("value", [
        "",        # empty string
        "   ",     # only whitespace
    ])
    def test_empty_raises(self, value):
        with pytest.raises(argparse.ArgumentTypeError):
            parse_host(value)

    def test_returns_tuple(self):
        result = parse_host("192.168.1.10")
        assert isinstance(result, tuple)
        assert len(result) == 2

    def test_port_is_int(self):
        _, port = parse_host("192.168.1.10:5000")
        assert isinstance(port, int)


# ---------------------------------------------------------------------------
# build_parser defaults
# ---------------------------------------------------------------------------

class TestBuildParser:
    def setup_method(self):
        self.parser = build_parser()

    # --- defaults ---

    def test_defaults_older_than(self):
        args = self.parser.parse_args([])
        assert args.older_than == DEFAULT_MAX_AGE

    def test_defaults_sort_by(self):
        args = self.parser.parse_args([])
        assert args.sort_by == "date"

    def test_defaults_connection(self):
        args = self.parser.parse_args([])
        assert args.host is None
        assert args.serial is None

    def test_defaults_flags_false(self):
        args = self.parser.parse_args([])
        assert args.reverse is False
        assert args.yes is False
        assert args.verbose == 0
        assert args.purge_unknown is False

    # --- serial flags ---

    def test_serial_short_flag_no_device(self):
        """'-s' without a device path sets serial to '' (auto-detect sentinel)."""
        args = self.parser.parse_args(["-s"])
        assert args.serial == ""
        assert args.host is None

    def test_serial_long_flag_no_device(self):
        args = self.parser.parse_args(["--serial"])
        assert args.serial == ""

    def test_port_alias_no_device(self):
        """'--port' is an alias for '--serial'."""
        args = self.parser.parse_args(["--port"])
        assert args.serial == ""

    def test_serial_with_device(self):
        args = self.parser.parse_args(["-s", "/dev/ttyUSB0"])
        assert args.serial == "/dev/ttyUSB0"

    def test_serial_long_with_device(self):
        args = self.parser.parse_args(["--serial", "/dev/ttyUSB0"])
        assert args.serial == "/dev/ttyUSB0"

    # --- TCP flags ---

    def test_tcp_short_flag_no_host(self):
        """'-t' without a host defaults to ('localhost', 4403)."""
        args = self.parser.parse_args(["-t"])
        assert args.host == ("localhost", 4403)
        assert args.serial is None

    def test_tcp_long_flag_no_host(self):
        args = self.parser.parse_args(["--tcp"])
        assert args.host == ("localhost", 4403)

    def test_host_alias_no_host(self):
        """'--host' is an alias for '--tcp'."""
        args = self.parser.parse_args(["--host"])
        assert args.host == ("localhost", 4403)

    def test_tcp_with_hostname(self):
        args = self.parser.parse_args(["-t", "192.168.1.10"])
        host, port = args.host
        assert host == "192.168.1.10"
        assert port == 4403

    def test_tcp_with_host_and_port(self):
        args = self.parser.parse_args(["-t", "192.168.1.10:5000"])
        host, port = args.host
        assert host == "192.168.1.10"
        assert port == 5000

    def test_tcp_with_url(self):
        args = self.parser.parse_args(["-t", "http://192.168.1.10:5000"])
        host, port = args.host
        assert host == "192.168.1.10"
        assert port == 5000

    # --- mutual exclusion ---

    def test_serial_and_tcp_are_exclusive(self):
        with pytest.raises(SystemExit):
            self.parser.parse_args(["-s", "-t"])

    def test_older_than_and_purge_unknown_are_exclusive(self):
        with pytest.raises(SystemExit):
            self.parser.parse_args(["--older-than", "7d", "--purge-unknown"])

    # --- other flags ---

    def test_older_than_override(self):
        args = self.parser.parse_args(["--older-than", "3d"])
        assert args.older_than == timedelta(days=3)

    def test_purge_unknown_flag(self):
        args = self.parser.parse_args(["--purge-unknown"])
        assert args.purge_unknown is True

    def test_sort_by_choices(self):
        for choice in ["date", "name", "id", "hardware"]:
            args = self.parser.parse_args(["--sort-by", choice])
            assert args.sort_by == choice

    def test_sort_by_invalid(self):
        with pytest.raises(SystemExit):
            self.parser.parse_args(["--sort-by", "invalid"])

    def test_reverse_flag(self):
        args = self.parser.parse_args(["--reverse"])
        assert args.reverse is True

    def test_verbose_single_v(self):
        args = self.parser.parse_args(["-v"])
        assert args.verbose == 1

    def test_verbose_double_v(self):
        args = self.parser.parse_args(["-vv"])
        assert args.verbose == 2

    def test_verbose_stacked(self):
        args = self.parser.parse_args(["-v", "-v"])
        assert args.verbose == 2
