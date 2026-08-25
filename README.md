# blitzjson

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Drop-in replacement for Python's `json` module with native Django type support. Built with Rust via PyO3 for maximum performance.

## Features

- **Drop-in replacement**: Same API as `json.dumps()`, `json.loads()`, etc.
- **Native Django types**: Handles `datetime`, `date`, `time`, `timedelta`, `UUID`, `Decimal`, `QuerySet`, `Model`, `Promise` without custom encoders
- **Rust-powered**: Direct serialization to JSON buffer via CPython FFI, no intermediate Python objects
- **Zero dependencies**: No runtime Python dependencies required

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

vs `json` + `DjangoJSONEncoder` (CPython 3.14, Linux x86_64):

```
Benchmark                           json+DJE    blitzjson    Speedup
------------------------------------------------------------------------
Simple dict (4 fields)                 2.5µs        0.4µs      6.6x
Nested dict (deep)                     5.1µs        1.6µs      3.2x
Large list (1000 items)              697.3µs      333.8µs      2.1x
Datetime dict                         10.9µs        5.8µs      1.9x
UUID dict                              8.3µs        4.2µs      2.0x
Decimal dict                           5.1µs        1.3µs      4.1x
Mixed dict (all types)                 7.8µs        3.6µs      2.2x
```

## Requirements

- Python 3.11+
- No Rust installation required (pre-built wheels)

## License

MIT - [Ricardo Robles Fernández](https://github.com/rroblf01)
