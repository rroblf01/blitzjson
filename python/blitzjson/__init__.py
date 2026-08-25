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
    dumps as _rust_dumps,
    loads as _rust_loads,
    dump as _rust_dump,
    dumpb,
    dump_queryset,
    dump_queryset_bytes,
    stream_dump_queryset,
    stream_dump_queryset_jsonl,
)
from blitzjson._helpers import JSONDecodeError, JSONEncoder, JSONDecoder


def dumps(obj, skipkeys=False, ensure_ascii=True, check_circular=True,
          allow_nan=True, cls=None, indent=None, separators=None,
          default=None, sort_keys=False, **kw):
    """Serialize obj to a JSON formatted str."""
    return _rust_dumps(obj, skipkeys=skipkeys, ensure_ascii=ensure_ascii,
                       check_circular=check_circular, allow_nan=allow_nan,
                       cls=cls, indent=indent, separators=separators,
                       default=default, sort_keys=sort_keys)


def loads(s, cls=None, object_hook=None, parse_float=None,
          parse_int=None, parse_constant=None, object_pairs_hook=None,
          strict=True, **kw):
    """Deserialize s (a str, bytes or bytearray) to a Python object."""
    try:
        return _rust_loads(s, object_hook=object_hook,
                           object_pairs_hook=object_pairs_hook,
                           parse_float=parse_float, parse_int=parse_int)
    except ValueError as e:
        msg = str(e)
        doc = s if isinstance(s, str) else str(s)
        raise JSONDecodeError(msg, doc, (0, 0)) from e


def dump(obj, fp, skipkeys=False, ensure_ascii=True, check_circular=True,
         allow_nan=True, cls=None, indent=None, separators=None,
         default=None, sort_keys=False, **kw):
    """Serialize obj as JSON to fp."""
    s = dumps(obj, skipkeys=skipkeys, ensure_ascii=ensure_ascii,
              check_circular=check_circular, allow_nan=allow_nan,
              cls=cls, indent=indent, separators=separators,
              default=default, sort_keys=sort_keys)
    fp.write(s)


def load(fp, cls=None, object_hook=None, parse_float=None,
         parse_int=None, parse_constant=None, object_pairs_hook=None,
         strict=True, **kw):
    """Deserialize fp to a Python object."""
    content = fp.read()
    return loads(content, cls=cls, object_hook=object_hook,
                 parse_float=parse_float, parse_int=parse_int,
                 object_pairs_hook=object_pairs_hook, strict=strict)

# Alias dumpb to dump_bytes for consistency
dump_bytes = dumpb

__version__ = "0.1.0"
__all__ = [
    # Core functions
    "dumps",
    "loads",
    "dump",
    "load",
    # blitzjson extensions
    "dumpb",
    "dump_bytes",
    "dump_queryset",
    "dump_queryset_bytes",
    "stream_dump_queryset",
    "stream_dump_queryset_jsonl",
    # Classes
    "JSONDecodeError",
    "JSONEncoder",
    "JSONDecoder",
    # Django integration
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
