# blitzjson

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Drop-in replacement for Python's `json` module with native Django type support. Built with Rust via PyO3 for maximum performance.

## Features

- **Drop-in replacement**: Same API as `json.dumps()`, `json.loads()`, etc.
- **Native Django types**: Handles `datetime`, `date`, `time`, `timedelta`, `UUID`, `Decimal`, `QuerySet`, `Model`, `Promise` without custom encoders
- **Rust-powered**: Direct serialization to JSON buffer via CPython FFI, no intermediate Python objects
- **Zero dependencies**: No runtime Python dependencies required
- **Full json API**: `ensure_ascii`, `indent`, `sort_keys`, `allow_nan`, `default` (recursive)
- **Streaming**: `stream_dump_queryset()` for memory-efficient large QuerySet serialization
- **Django integration**: `BlitzJsonResponse`, `BlitzJSONEncoder`, `install()` monkey-patch

## Installation

```bash
pip install blitzjson
```

## Quick Start

```python
# Before
import json
from datetime import datetime

json.dumps({"created": datetime.now()})  # TypeError!

# After
import blitzjson as json

json.dumps({"created": datetime.now()})  # Works!
```

## Monkey-patching (try before you commit)

If you want to test blitzjson in your existing project without changing any `import json` statements, use the `install()` function to monkey-patch Python's built-in `json` module:

```python
# In your settings.py or at the top of manage.py
import blitzjson
blitzjson.install()

# Now ALL code that uses `import json` will use blitzjson instead
import json
json.dumps({"created": datetime.now()})  # Works! No TypeError!
```

Or test it interactively:

```python
>>> import blitzjson
>>> blitzjson.install()
>>> import json
>>> json.dumps({"created": datetime.now(), "uuid": uuid4()})
'{"created": "2024-01-15T10:30:45+00:00", "uuid": "550e8400-e29b-41d4-a716-446655440000"}'

>>> # Revert when done testing
>>> blitzjson.uninstall()
```

## Supported Types

| Python Type | JSON Output | Example |
|---|---|---|
| `datetime` | ISO 8601 string | `"2024-01-15T10:30:45Z"` |
| `date` | ISO date string | `"2024-01-15"` |
| `time` | ISO time string | `"10:30:45"` |
| `timedelta` | ISO 8601 duration | `"P1DT2H3M4S"` |
| `UUID` | Hyphenated string | `"550e8400-e29b-41d4-a716-446655440000"` |
| `Decimal` | String (precision-safe) | `"123.456789012345678901"` |
| `bytes` | Base64 string | `"aGVsbG8="` |
| `set/frozenset` | Array | `[1, 2, 3]` |
| `enum` | Value | `"value"` |
| `dataclass` | Object | `{"field": "value"}` |

### Django-Specific

| Django Type | JSON Output |
|---|---|
| `QuerySet` | Array of model dicts |
| `Model` | Dict of field values |
| `Promise` (lazy strings) | String |

## API

### `dumps(obj, **kwargs)`

```python
import blitzjson as json

# Standard usage
json.dumps({"key": "value"})

# With Django types
json.dumps({"created": datetime.now(), "uuid": uuid4()})
```

### `loads(s, **kwargs)`

```python
obj = json.loads('{"key": "value"}')

# With object_hook
obj = json.loads('{"key": "value"}', object_hook=lambda d: {k.upper(): v for k, v in d.items()})
```

### `dump(obj, fp, **kwargs)` / `load(fp, **kwargs)`

File-based serialization/deserialization.

### `dumpb(obj, pretty=False)`

Serialize to bytes (faster than `dumps` for network responses).

### `dump_queryset(queryset)` / `dump_queryset_bytes(queryset)`

Optimized serialization for Django QuerySets.

## Benchmarks

vs `json` + `DjangoJSONEncoder` (CPython 3.14, Linux x86_64, consumer hardware):

> **Note:** Benchmarks run on consumer hardware (AMD Ryzen 9 7900X). CI runners
> (Intel Xeon server CPUs) may show different results due to lower single-thread
> performance. For stable benchmarks, run locally with `uv run python benchmarks/bench_serialization.py`.

### dumps

```
Benchmark                           json+DJE    blitzjson    Speedup
------------------------------------------------------------------------
Simple dict (4 fields)                 2.5µs        0.8µs      3.0x
Nested dict (deep)                     5.4µs        1.8µs      3.0x
Large list (1000 items)              752.1µs      193.2µs      3.9x
String-heavy dict (20 keys)            4.8µs        1.5µs      3.1x
Datetime dict                         10.3µs        3.6µs      2.8x
UUID dict                              7.3µs        3.9µs      1.9x
Decimal dict                           4.4µs        1.6µs      2.7x
Mixed dict (all types)                 7.3µs        2.8µs      2.6x
```

### loads

```
Benchmark                               json    blitzjson    Speedup
------------------------------------------------------------------------
Simple dict (4 fields)                 1.5µs        0.8µs      2.0x
Nested dict (deep)                     5.1µs        3.2µs      1.6x
Large list (1000 items)              716.0µs      421.2µs      1.7x
String-heavy dict (20 keys)            4.3µs        2.7µs      1.6x
```

### Django Response

```
Benchmark                             JsonResponse      BlitzJson    Speedup
------------------------------------------------------------------------
API response (50 users)                 44.9µs         19.6µs      2.3x
```

## Requirements

- Python 3.10+
- No Rust installation required (pre-built wheels)

## License

MIT - [Ricardo Robles Fernández](https://github.com/rroblf01)
