# Changelog

## [0.1.0] - 2026-08-25

### Added

- Drop-in replacement for Python's `json` module (`dumps`, `loads`, `dump`, `load`)
- `dumpb` / `dump_bytes` for bytes serialization
- Native serialization of `datetime`, `date`, `time`, `timedelta` (ISO 8601)
- Native serialization of `UUID`, `Decimal` (as quoted strings to preserve precision)
- Native serialization of `set`, `frozenset`, `bytes`, `bytearray`
- Native serialization of `tuple`, `enum`, `dataclass`
- Django `QuerySet` and `Model` serialization
- Django `Promise` (lazy strings) support
- `dump_queryset` / `dump_queryset_bytes` for optimized QuerySet serialization
- `stream_dump_queryset` / `stream_dump_queryset_jsonl` for memory-efficient streaming
- `install()` / `uninstall()` to monkey-patch Python's built-in `json` module
- `BlitzJsonResponse` drop-in replacement for Django's `JsonResponse`
- `BlitzJSONEncoder` compatible with Django REST Framework
- `JSONEncoder`, `JSONDecoder`, `JSONDecodeError` (subclassable Python classes)
- `sort_keys`, `ensure_ascii`, `indent`, `separators`, `allow_nan`, `default` (recursive) support
- `object_hook`, `object_pairs_hook`, `parse_float`, `parse_int` for deserialization
- Multi-platform CI: Linux (x86_64), macOS (x86_64 + aarch64), Windows (x64)
- Python 3.10 through 3.14 support via ABI3 stable ABI
- Benchmarks vs stdlib `json` + `DjangoJSONEncoder` (2-4x faster)
