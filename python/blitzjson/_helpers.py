"""blitzjson._helpers - Python classes for json module compatibility.

These classes are implemented in Python so users can subclass them.
They delegate to the Rust core for actual serialization/deserialization.
"""

from blitzjson._core import (
    dumps as _rust_dumps,
    loads as _rust_loads,
)


class JSONDecodeError(ValueError):
    """Decode error with msg, doc, and pos attributes.

    Compatible with json.JSONDecodeError. Code that catches
    json.JSONDecodeError will also catch this.
    """

    def __init__(self, msg, doc, pos):
        super().__init__(msg)
        self.msg = msg
        self.doc = doc
        self.pos = pos

    def __reduce__(self):
        return (self.__class__, (self.msg, self.doc, self.pos))

    def __str__(self):
        return f'{self.msg} at row {self.pos[0]}, column {self.pos[1]} (char {self.pos[1]})'

    def __repr__(self):
        return f'{self.__class__.__name__}({self.msg!r}, {self.doc!r}, {self.pos!r})'


class JSONEncoder:
    """Encodes Python objects to JSON strings.

    Subclass and override default() to handle additional types.
    Compatible with json.JSONEncoder.
    """

    item_separator = ', '
    key_separator = ': '

    def __init__(self, *, skipkeys=False, ensure_ascii=True, check_circular=True,
                 allow_nan=True, sort_keys=False, indent=None, separators=None,
                 default=None):
        self.skipkeys = skipkeys
        self.ensure_ascii = ensure_ascii
        self.check_circular = check_circular
        self.allow_nan = allow_nan
        self.sort_keys = sort_keys
        self.indent = indent
        self._default = default  # Use _default to avoid shadowing the method
        if separators is not None:
            self.item_separator, self.key_separator = separators
        elif indent is None:
            self.item_separator = ', '
            self.key_separator = ': '
        else:
            self.item_separator = ','
            self.key_separator = ': '

    def default(self, o):
        """Override this method to handle additional types.

        Called for objects that can't be serialized natively.
        Must return a JSON-serializable object or raise TypeError.
        """
        raise TypeError(f'Object of type {type(o).__name__} is not JSON serializable')

    def encode(self, o):
        """Return a JSON string representation of o."""
        # Check if default was overridden by subclass
        default_func = None
        if 'default' in type(self).__dict__:
            # Class has its own default method (not inherited from JSONEncoder)
            default_func = self.default
        elif self._default is not None:
            # default was passed as a parameter to __init__
            default_func = self._default

        return _rust_dumps(
            o,
            skipkeys=self.skipkeys,
            ensure_ascii=self.ensure_ascii,
            check_circular=self.check_circular,
            allow_nan=self.allow_nan,
            sort_keys=self.sort_keys,
            indent=self.indent,
            separators=(self.item_separator, self.key_separator) if self.indent is None else None,
            default=default_func,
        )

    def iterencode(self, o, _one_shot=False):
        """Iteratively encode an object to JSON string chunks."""
        yield self.encode(o)


class BlitzJSONEncoder(JSONEncoder):
    """JSON encoder that uses blitzjson for serialization.

    Compatible with Django REST Framework and any code that uses
    json.JSONEncoder as a base class.

    Usage:
        import json
        from blitzjson import BlitzJSONEncoder

        json.dumps(data, cls=BlitzJSONEncoder)
    """

    def default(self, o):
        from datetime import datetime, date, time, timedelta
        from uuid import UUID
        from decimal import Decimal

        if isinstance(o, datetime):
            return o.isoformat()
        elif isinstance(o, date):
            return o.isoformat()
        elif isinstance(o, time):
            return o.isoformat()
        elif isinstance(o, timedelta):
            total_seconds = o.total_seconds()
            days = int(total_seconds // 86400)
            remaining = total_seconds % 86400
            hours = int(remaining // 3600)
            minutes = int((remaining % 3600) // 60)
            secs = int(remaining % 60)
            micros = int((remaining - secs) * 1_000_000)
            result = "P"
            if days:
                result += f"{days}D"
            result += "T"
            if hours:
                result += f"{hours}H"
            if minutes:
                result += f"{minutes}M"
            if micros:
                result += f"{secs}.{micros:06d}S"
            elif secs:
                result += f"{secs}S"
            elif result == "PT":
                result = "PT0S"
            return result
        elif isinstance(o, (Decimal, UUID)):
            return str(o)
        return super().default(o)


class JSONDecoder:
    """Decode JSON strings to Python objects.

    Compatible with json.JSONDecoder.
    """

    def __init__(self, *, object_hook=None, parse_float=None,
                 parse_int=None, parse_constant=None, strict=True,
                 object_pairs_hook=None):
        self.object_hook = object_hook
        self.parse_float = parse_float
        self.parse_int = parse_int
        self.parse_constant = parse_constant
        self.strict = strict
        self.object_pairs_hook = object_pairs_hook

    def decode(self, s):
        """Return the Python representation of s (a str instance)."""
        return _rust_loads(
            s,
            object_hook=self.object_hook,
            object_pairs_hook=self.object_pairs_hook,
            parse_float=self.parse_float,
            parse_int=self.parse_int,
        )

    def raw_decode(self, s, idx=0):
        """Decode s starting from idx, return (obj, end_idx).

        Compatible with json.JSONDecoder.raw_decode().
        """
        if isinstance(s, bytes):
            s = s.decode('utf-8')
        if isinstance(s, memoryview):
            s = bytes(s).decode('utf-8')
        s = s[idx:]
        obj = self.decode(s)
        return obj, len(s)
