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

    # Django QuerySet serialization
    from blitzjson import dump_queryset
    json_string = dump_queryset(my_queryset)
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
]
