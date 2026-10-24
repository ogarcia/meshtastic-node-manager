## Application

app-name = Meshtastic Node Manager
app-tagline = Manage your device's nodedb
about = Inspect and clean the nodedb of a Meshtastic device.

## Command-line help

help-usage = Usage:
heading-commands = Commands
heading-connection = Connection
heading-selection = Selection (default: --unknown)
heading-output = Output
heading-action = Action
heading-arguments = Arguments
cmd-list = Show the device nodedb (default command)
cmd-clean = Remove partially-known or stale nodes
cmd-remove = Remove specific nodes
cmd-favorite = Mark nodes as favorites
cmd-unfavorite = Remove nodes from favorites
cmd-ignore = Ignore nodes
cmd-unignore = Stop ignoring nodes
cmd-set-time = Set the device clock to the system time
help-serial = Connect via serial. DEVICE (e.g. /dev/ttyUSB0) is auto-detected when omitted. Never falls back to TCP
help-tcp = Connect via TCP. HOST may be host, host:port or http://host:port; defaults to localhost:4403. Never falls back to serial
help-unknown = Select partially-known nodes (no complete NodeInfo). This is the default when no other selection is given
help-older-than = Select nodes not heard for longer than DURATION (e.g. 30m, 24h, 7d, 2w). Combined with --unknown, both must match
help-include-never = With --older-than, also select nodes with no last-heard time
help-nodes = Node ids: !aabbccdd, 0xaabbccdd or a decimal number
help-sort-by = Column to sort the table by
help-reverse = Reverse the sort order (date: oldest first; text: Z→A)
help-verbose = Increase verbosity (-v connection details, -vv debug output)
help-dry-run = Show what would be removed without changing anything
help-yes = Remove without asking for confirmation
help-help = Print help
help-version = Print version
help-examples =
    Examples:
      meshtastic-node-manager                            Show the nodedb, auto-detecting the device
      meshtastic-node-manager -t 192.168.1.10            Show the nodedb of a device reachable via TCP
      meshtastic-node-manager clean -n                   Show which partially-known nodes would be removed
      meshtastic-node-manager clean -o 30d               Remove nodes not heard for 30 days
      meshtastic-node-manager clean -u -o 7d             Remove partially-known nodes not heard for 7 days
      meshtastic-node-manager remove '!aabbccdd'         Remove one specific node
      meshtastic-node-manager unfavorite '!aabbccdd'     Unmark a favorite so that it can be removed

    Favorites and ignored nodes are never removed.
    Use "meshtastic-node-manager <COMMAND> --help" for the options of each command.

## Argument errors

error-duration = invalid duration '{ $value }'; use e.g. 30m, 24h, 7d or 2w
error-host = invalid host '{ $value }'; use e.g. 192.168.1.10 or 192.168.1.10:4403
error-node-id = invalid node id '{ $value }'; use e.g. !aabbccdd

## Connection

connecting-serial = Connecting via serial to { $device }…
connecting-tcp = Connecting via TCP to { $address }…
serial-multiple = Multiple serial ports found ({ $ports }); using { $port }. Use --serial DEVICE to choose another.
serial-none-fallback = No serial device detected. Trying TCP on localhost…
error = Error:
error-connect = could not connect to the device
error-no-serial = no serial device found; connect one or use --tcp
error-config-timeout = timed out waiting for the device nodedb
error-connection-closed = the device closed the connection
error-no-my-info = the device did not report its own node number
error-rejected = the device rejected the request ({ $reason })
error-set-time = Could not set the device time:
time-set = Device clock set to the system time.
time-set-unconfirmed = Device time sent, but the device did not acknowledge it.

## Node table

nodedb-summary = { $count ->
    [one] The device has { $count } node in its nodedb.
   *[other] The device has { $count } nodes in its nodedb.
}
nodedb-empty = The device has no other nodes in its nodedb.
to-delete = { $count ->
    [0] No nodes to remove
    [one] { $count } node to remove
   *[other] { $count } nodes to remove
}
criteria-unknown = partially-known nodes
criteria-older = not heard for more than { $age }
criteria-unknown-older = partially-known and not heard for more than { $age }
criteria-include-never = (including never heard)
criteria-explicit = requested nodes
col-id = Node ID
col-long-name = Long name
col-short-name = Short
col-hardware = Hardware
col-last-heard = Last heard
col-ago = Ago
never-heard = never
legend-own = own node
legend-delete = to remove
legend-favorite = favorite
legend-ignored = ignored
legend-unknown = partially known

## Notes

skip-not-found = { $id } is not in the nodedb.
skip-own = { $id } is the device's own node and cannot be removed.
skip-favorite = { $id } is a favorite and will not be removed; unmark it first with the unfavorite command.
skip-ignored = { $id } is ignored and will not be removed, as that would drop the ignore flag; use the unignore command first.
note-protected = { $count ->
    [one] { $count } matching node is kept because it is a favorite or ignored.
   *[other] { $count } matching nodes are kept because they are favorites or ignored.
}
note-never-heard = { $count ->
    [one] { $count } node has no last-heard time and was kept.
   *[other] { $count } nodes have no last-heard time and were kept.
} The device probably had no valid clock when it heard them. Use --include-never to select them too, and the set-time command to give the device the current time.

## Removal

all-clean = All clean!
nothing-to-delete = Nothing to remove.
dry-run = Dry run:
dry-run-summary = { $count ->
    [one] { $count } node would be removed. Nothing was changed.
   *[other] { $count } nodes would be removed. Nothing was changed.
}
confirm = { $count ->
    [one] Remove { $count } node?
   *[other] Remove { $count } nodes?
}
confirm-hint = [y/N]
# Comma-separated answers accepted as "yes" (lowercase).
confirm-yes-answers = y, yes
cancelled = Cancelled. No nodes were removed.
removed = Removed { $id } ({ $name })
removed-unconfirmed = Removal of { $id } ({ $name }) sent, but not acknowledged
remove-failed = Could not remove { $id }: { $error }
result = { $total ->
    [one] { $removed } of { $total } node removed.
   *[other] { $removed } of { $total } nodes removed.
}

## Node flags

flag-own = { $id } is the device's own node; skipped.
flag-unconfirmed = Change for { $id } ({ $name }) sent, but not acknowledged
flag-failed = Could not change { $id }: { $error }
favorite-already = { $id } is already a favorite.
favorite-not = { $id } is not a favorite.
favorite-added = Added { $id } ({ $name }) to favorites
favorite-removed = Removed { $id } ({ $name }) from favorites
ignore-already = { $id } is already ignored.
ignore-not = { $id } is not ignored.
ignore-added = Now ignoring { $id } ({ $name })
ignore-removed = No longer ignoring { $id } ({ $name })
