"""Internationalization — exposes the _() translation function.

Reads the locale from the environment (LANG, LC_ALL, LANGUAGE, etc.) and
loads the corresponding .mo file from the locale/ directory bundled with the
package.  Falls back to a no-op translation when no compiled catalogue is
found, so the English source strings are returned as-is.
"""

import gettext
from pathlib import Path

_LOCALE_DIR = Path(__file__).parent / "locale"
_DOMAIN = "meshtastic-node-cleaner"

try:
    _ = gettext.translation(_DOMAIN, localedir=_LOCALE_DIR).gettext
except FileNotFoundError:
    _ = gettext.NullTranslations().gettext
