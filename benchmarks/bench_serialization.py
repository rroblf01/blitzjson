"""Benchmarks comparing blitzjson vs json vs orjson."""

import json
import timeit
import statistics
from datetime import datetime, date, time, timedelta, timezone
from uuid import UUID, uuid4
from decimal import Decimal


def benchmark(func, number=1000, rounds=3):
    """Run benchmark and return median time in microseconds."""
    times = timeit.repeat(func, number=number, repeat=rounds)
    per_op = [t / number * 1_000_000 for t in times]
    return {
        "median_us": statistics.median(per_op),
        "min_us": min(per_op),
        "max_us": max(per_op),
    }


# ── Test data ──────────────────────────────────────────────────────

SIMPLE_DICT = {"name": "John", "age": 30, "active": True, "score": 9.5}

NESTED_DICT = {
    "user": {
        "id": 1,
        "name": "Alice",
        "email": "alice@example.com",
        "profile": {
            "bio": "Software developer",
            "location": "Madrid",
            "website": "https://alice.dev",
        },
    },
    "posts": [
        {"title": "First post", "content": "Hello world", "likes": 42},
        {"title": "Second post", "content": "Python tips", "likes": 128},
        {"title": "Third post", "content": "Rust speed", "likes": 256},
    ],
    "settings": {"theme": "dark", "language": "es", "notifications": True},
}

LARGE_LIST = [{"id": i, "value": f"item_{i}", "score": i * 1.1} for i in range(1000)]

DATETIME_DICT = {
    "created_at": datetime(2024, 1, 15, 10, 30, 45, 123456, tzinfo=timezone.utc),
    "updated_at": datetime(2024, 6, 20, 14, 0, 0),
    "birth_date": date(1990, 5, 15),
    "alarm_time": time(7, 30),
    "duration": timedelta(days=3, hours=2, minutes=30),
}

UUID_DICT = {
    "id": uuid4(),
    "session_id": uuid4(),
    "tracking_id": uuid4(),
    "request_id": uuid4(),
}

DECIMAL_DICT = {
    "price": Decimal("29.99"),
    "total": Decimal("1234.56"),
    "tax": Decimal("0.21"),
    "discount": Decimal("15.50"),
}

MIXED_DICT = {
    "id": 42,
    "name": "Product",
    "price": Decimal("29.99"),
    "created": datetime(2024, 1, 15, tzinfo=timezone.utc),
    "uuid": uuid4(),
    "tags": ["python", "rust", "json"],
    "active": True,
    "metadata": None,
}

# String-heavy data
STRING_DICT = {f"key_{i}": f"value_{i}" for i in range(20)}

# Pre-serialized JSON strings for loads benchmarks
SIMPLE_JSON = json.dumps(SIMPLE_DICT)
NESTED_JSON = json.dumps(NESTED_DICT)
LARGE_JSON = json.dumps(LARGE_LIST)
STRING_JSON = json.dumps(STRING_DICT)


# ── DjangoJSONEncoder for json ─────────────────────────────────────

class DjangoJSONEncoder(json.JSONEncoder):
    def default(self, o):
        if isinstance(o, datetime):
            r = o.isoformat(sep="T", timespec="milliseconds" if o.microsecond // 1000 else "seconds")
            if r.endswith("+00:00"):
                r = r.removesuffix("+00:00") + "Z"
            return r
        elif isinstance(o, date):
            return o.isoformat()
        elif isinstance(o, time):
            r = o.isoformat(timespec="milliseconds" if o.microsecond // 1000 else "seconds")
            return r
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


# ── Run benchmarks ─────────────────────────────────────────────────

def run_benchmarks():
    import blitzjson

    try:
        import orjson
        has_orjson = True
    except ImportError:
        has_orjson = False
        print("orjson not installed, skipping orjson benchmarks\n")

    # ── dumps benchmarks ───────────────────────────────────────
    print("=" * 75)
    print("DUMPS benchmarks")
    print("=" * 75)
    print(f"{'Benchmark':<35} {'json+DJE':>12} {'blitzjson':>12} {'Speedup':>10}")
    print("-" * 75)

    dumps_benchmarks = [
        ("Simple dict (4 fields)", lambda: json.dumps(SIMPLE_DICT, cls=DjangoJSONEncoder),
         lambda: blitzjson.dumps(SIMPLE_DICT)),
        ("Nested dict (deep)", lambda: json.dumps(NESTED_DICT, cls=DjangoJSONEncoder),
         lambda: blitzjson.dumps(NESTED_DICT)),
        ("Large list (1000 items)", lambda: json.dumps(LARGE_LIST, cls=DjangoJSONEncoder),
         lambda: blitzjson.dumps(LARGE_LIST)),
        ("String-heavy dict (20 keys)", lambda: json.dumps(STRING_DICT, cls=DjangoJSONEncoder),
         lambda: blitzjson.dumps(STRING_DICT)),
        ("Datetime dict", lambda: json.dumps(DATETIME_DICT, cls=DjangoJSONEncoder),
         lambda: blitzjson.dumps(DATETIME_DICT)),
        ("UUID dict", lambda: json.dumps(UUID_DICT, cls=DjangoJSONEncoder),
         lambda: blitzjson.dumps(UUID_DICT)),
        ("Decimal dict", lambda: json.dumps(DECIMAL_DICT, cls=DjangoJSONEncoder),
         lambda: blitzjson.dumps(DECIMAL_DICT)),
        ("Mixed dict (all types)", lambda: json.dumps(MIXED_DICT, cls=DjangoJSONEncoder),
         lambda: blitzjson.dumps(MIXED_DICT)),
    ]

    if has_orjson:
        dumps_benchmarks += [
            ("Simple dict (orjson)", lambda: orjson.dumps(SIMPLE_DICT),
             lambda: blitzjson.dumps(SIMPLE_DICT)),
            ("Mixed dict (orjson)", lambda: orjson.dumps(MIXED_DICT, default=str),
             lambda: blitzjson.dumps(MIXED_DICT)),
        ]

    for name, json_fn, blitz_fn in dumps_benchmarks:
        json_result = benchmark(json_fn)
        blitz_result = benchmark(blitz_fn)
        json_us = json_result["median_us"]
        blitz_us = blitz_result["median_us"]
        speedup = json_us / blitz_us
        print(f"{name:<35} {json_us:>10.1f}µs {blitz_us:>10.1f}µs {speedup:>8.1f}x")

    # ── loads benchmarks ───────────────────────────────────────
    print()
    print("=" * 75)
    print("LOADS benchmarks")
    print("=" * 75)
    print(f"{'Benchmark':<35} {'json':>12} {'blitzjson':>12} {'Speedup':>10}")
    print("-" * 75)

    loads_benchmarks = [
        ("Simple dict (4 fields)", lambda: json.loads(SIMPLE_JSON),
         lambda: blitzjson.loads(SIMPLE_JSON)),
        ("Nested dict (deep)", lambda: json.loads(NESTED_JSON),
         lambda: blitzjson.loads(NESTED_JSON)),
        ("Large list (1000 items)", lambda: json.loads(LARGE_JSON),
         lambda: blitzjson.loads(LARGE_JSON)),
        ("String-heavy dict (20 keys)", lambda: json.loads(STRING_JSON),
         lambda: blitzjson.loads(STRING_JSON)),
    ]

    if has_orjson:
        loads_benchmarks += [
            ("Simple dict (orjson)", lambda: orjson.loads(SIMPLE_JSON),
             lambda: blitzjson.loads(SIMPLE_JSON)),
        ]

    for name, json_fn, blitz_fn in loads_benchmarks:
        json_result = benchmark(json_fn)
        blitz_result = benchmark(blitz_fn)
        json_us = json_result["median_us"]
        blitz_us = blitz_result["median_us"]
        speedup = json_us / blitz_us
        print(f"{name:<35} {json_us:>10.1f}µs {blitz_us:>10.1f}µs {speedup:>8.1f}x")


if __name__ == "__main__":
    run_benchmarks()
