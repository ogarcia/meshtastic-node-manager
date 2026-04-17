#!/usr/bin/env bash
# Compile all gettext .po catalogues into binary .mo files.
#
# .mo files are not tracked in git; run this script after cloning or after
# editing any .po file.
#
# Usage:
#   scripts/compile-translations.sh

set -euo pipefail

LOCALE_DIR="$(cd "$(dirname "$0")/../meshtastic_node_cleaner/locale" && pwd)"

if ! command -v msgfmt &> /dev/null; then
    echo "Error: msgfmt not found. Install gettext (e.g. 'apt install gettext' or 'brew install gettext')." >&2
    exit 1
fi

compiled=0
failed=0

for po in "$LOCALE_DIR"/*/LC_MESSAGES/*.po; do
    mo="${po%.po}.mo"
    lang=$(basename "$(dirname "$(dirname "$po")")")
    if msgfmt "$po" -o "$mo"; then
        echo "  [ok] $lang"
        compiled=$(( compiled + 1 ))
    else
        echo "  [FAIL] $lang — msgfmt returned an error" >&2
        failed=$(( failed + 1 ))
    fi
done

echo ""
echo "$compiled catalogue(s) compiled successfully."
if [ "$failed" -gt 0 ]; then
    echo "$failed catalogue(s) failed." >&2
    exit 1
fi
