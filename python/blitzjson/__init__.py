"""blitzjson - Drop-in replacement for Python's json module with native Django type support.

Rust-powered JSON serialization via PyO3. Handles Django types natively:
- datetime, date, time, timedelta
- UUID, Decimal
- QuerySet, Model
- Promise (lazy strings)
- set, frozenset, bytes, enum, dataclass

Usage:
    import blitzjson as json

    # Exact same API as json
    data = json.dumps({"key": "value"})
    obj = json.loads(data)

    # With ensure_ascii, indent, sort_keys
    data = json.dumps({"ñ": "á"}, ensure_ascii=True)  # "\\u00f1": "\\u00e1"
    data = json.dumps({"b": 2, "a": 1}, sort_keys=True)  # {"a":1,"b":2}
    data = json.dumps({"a": 1}, indent=2)  # pretty printed

    # Django QuerySet serialization
    from blitzjson import dump_queryset
    json_string = dump_queryset(my_queryset)

    # Django integration
    from blitzjson.django import BlitzJsonResponse, install
    install()  # monkey-patch json module
"""

from blitzjson._core import (
    dumps,
    loads,
    dump,
    load,
    dumpb,
    dump_queryset,
    dump_queryset_bytes,
)

# Alias dumpb to dump_bytes for consistency
dump_bytes = dumpb

# Standard json module compatibility
JSONDecodeError = ValueError
JSONEncoder = None
JSONDecoder = None

__version__ = "0.1.0"
__all__ = [
    "dumps",
    "loads",
    "dump",
    "load",
    "dumpb",
    "dump_bytes",
    "dump_queryset",
    "dump_queryset_bytes",
    "JSONDecodeError",
    "JSONEncoder",
    "JSONDecoder",
    "install",
    "uninstall",
]


def install():
    """Monkey-patch Python's json module with blitzjson."""
    from blitzjson.django import install as _install
    _install()


def uninstall():
    """Revert monkey-patching done by install()."""
    from blitzjson.django import uninstall as _uninstall
    _uninstall()
