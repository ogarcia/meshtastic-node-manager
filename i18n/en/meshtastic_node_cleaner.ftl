## Application

app-name = Meshtastic Node Cleaner
app-tagline = Clean nodes from your device's nodedb
about = Remove stale or partially-known nodes from a Meshtastic device's nodedb.

## Command-line help

help-usage = Usage:
heading-connection = Connection
heading-selection = Selection (default: --unknown)
heading-output = Output
heading-action = Action
help-serial = Connect via serial. DEVICE (e.g. /dev/ttyUSB0) is auto-detected when omitted. Never falls back to TCP
help-tcp = Connect via TCP. HOST may be host, host:port or http://host:port; defaults to localhost:4403. Never falls back to serial
help-unknown = Select partially-known nodes (no complete NodeInfo). This is the default when no other selection is given
help-older-than = Select nodes not heard for longer than DURATION (e.g. 30m, 24h, 7d, 2w). Combined with --unknown, both must match
help-include-never = With --older-than, also select nodes with no last-heard time
help-remove = Remove exactly these nodes (e.g. !aabbccdd, 0xaabbccdd or a decimal number)
help-sort-by = Column to sort the table by
help-reverse = Reverse the sort order (date: oldest first; text: Z→A)
help-verbose = Increase verbosity (-v connection details, -vv debug output)
help-dry-run = Show what would be removed without changing anything
help-yes = Remove without asking for confirmation
help-set-time = Set the device clock to the system time before reading the nodedb
help-help = Print help
help-version = Print version
help-examples =
    Examples:
      meshtastic-node-cleaner                     Remove partially-known nodes, auto-detecting the device
      meshtastic-node-cleaner -n                  Only show what would be removed
      meshtastic-node-cleaner -s /dev/ttyUSB0     Use a specific serial device
      meshtastic-node-cleaner -t 192.168.1.10     Connect via TCP
      meshtastic-node-cleaner -o 30d              Remove nodes not heard for 30 days
      meshtastic-node-cleaner -u -o 7d            Remove partially-known nodes not heard for 7 days
      meshtastic-node-cleaner -r '!aabbccdd'      Remove one specific node

    Favorites and ignored nodes are never removed.

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
skip-favorite = { $id } is a favorite and will not be removed; unmark it first.
skip-ignored = { $id } is ignored and will not be removed; removing it would drop the ignore flag.
note-protected = { $count ->
    [one] { $count } matching node is kept because it is a favorite or ignored.
   *[other] { $count } matching nodes are kept because they are favorites or ignored.
}
note-never-heard = { $count ->
    [one] { $count } node has no last-heard time and was kept.
   *[other] { $count } nodes have no last-heard time and were kept.
} The device probably had no valid clock when it heard them. Use --include-never to select them too, and --set-time to give the device the current time.

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
