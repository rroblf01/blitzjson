"""Django-specific benchmarks for blitzjson."""

import json
import timeit
import statistics
import django
from django.conf import settings

if not settings.configured:
    settings.configure(
        INSTALLED_APPS=['django.contrib.contenttypes'],
        DATABASES={'default': {'ENGINE': 'django.db.backends.sqlite3', 'NAME': ':memory:'}}
    )
    django.setup()

from blitzjson.django import BlitzJsonResponse
from django.http import JsonResponse


def benchmark(func, number=1000, rounds=3):
    times = timeit.repeat(func, number=number, repeat=rounds)
    per_op = [t / number * 1_000_000 for t in times]
    return statistics.median(per_op)


# ── Test data ──────────────────────────────────────────────────────

API_RESPONSE = {
    "status": "success",
    "data": {
        "users": [
            {"id": i, "name": f"User {i}", "email": f"user{i}@example.com", "active": True}
            for i in range(50)
        ],
        "pagination": {
            "page": 1,
            "per_page": 50,
            "total": 1000,
            "total_pages": 20,
        }
    },
    "meta": {
        "request_id": "550e8400-e29b-41d4-a716-446655440000",
        "timestamp": "2024-01-15T10:30:45Z",
        "version": "1.0",
    }
}


def run_benchmarks():
    print("=" * 75)
    print("DJANGO RESPONSE BENCHMARKS")
    print("=" * 75)
    print(f"{'Benchmark':<35} {'JsonResponse':>14} {'BlitzJson':>14} {'Speedup':>10}")
    print("-" * 75)

    benchmarks = [
        ("API response (50 users)",
         lambda: JsonResponse(API_RESPONSE).content,
         lambda: BlitzJsonResponse(API_RESPONSE).content),
    ]

    for name, json_fn, blitz_fn in benchmarks:
        json_us = benchmark(json_fn)
        blitz_us = benchmark(blitz_fn)
        speedup = json_us / blitz_us
        print(f"{name:<35} {json_us:>12.1f}µs {blitz_us:>12.1f}µs {speedup:>8.1f}x")


if __name__ == "__main__":
    run_benchmarks()
