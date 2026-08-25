"""Django-specific benchmarks with realistic data."""
import json
import timeit
import statistics
from datetime import datetime, date, time, timedelta, timezone
from uuid import UUID, uuid4
from decimal import Decimal


def benchmark(func, number=1000, rounds=3):
    times = timeit.repeat(func, number=number, repeat=rounds)
    per_op = [t / number * 1_000_000 for t in times]
    return statistics.median(per_op)


# ── Django API responses ───────────────────────────────────────────

def django_user_list():
    return {
        'status': 'success',
        'data': {
            'users': [
                {'id': i, 'name': f'User {i}', 'email': f'user{i}@example.com',
                 'is_active': True, 'date_joined': '2024-01-15T10:30:45Z'}
                for i in range(50)
            ],
            'pagination': {'page': 1, 'per_page': 50, 'total': 1000, 'total_pages': 20}
        },
        'meta': {'request_id': '550e8400-e29b-41d4-a716-446655440000', 'version': '1.0'}
    }

def django_product_catalog():
    return {
        'products': [
            {'id': i, 'name': f'Product {i}', 'price': Decimal('29.99'),
             'sku': f'SKU-{i:06d}', 'in_stock': i % 3 != 0,
             'created': '2024-01-15T10:30:45Z'}
            for i in range(20)
        ],
        'categories': ['Electronics', 'Books', 'Clothing'],
        'total_count': 20
    }

def django_order_detail():
    return {
        'order_id': '550e8400-e29b-41d4-a716-446655440000',
        'customer': {'id': 1, 'name': 'Alice', 'email': 'alice@example.com'},
        'items': [
            {'product_id': i, 'name': f'Item {i}', 'quantity': i + 1,
             'unit_price': float(f'{(i+1)*19.99:.2f}'), 'total': float(f'{(i+1)*19.99:.2f}')}
            for i in range(5)
        ],
        'subtotal': float('299.95'),
        'tax': float('62.99'),
        'total': float('362.94'),
        'status': 'completed',
        'created_at': '2024-01-15T10:30:45Z',
        'updated_at': '2024-01-15T10:35:00Z'
    }


def run_benchmarks():
    import blitzjson

    try:
        import orjson
        has_orjson = True
    except ImportError:
        has_orjson = False

    benchmarks = [
        ("Django User List (50)", django_user_list),
        ("Django Order Detail", django_order_detail),
    ]

    print("=" * 75)
    print("DJANGO-SPECIFIC BENCHMARKS")
    print("=" * 75)
    print(f"{'Benchmark':<30} {'json+DJE':>12} {'blitzjson':>12} {'Speedup':>10}")
    print("-" * 75)

    for name, data_fn in benchmarks:
        data = data_fn()
        json_fn = lambda: json.dumps(data, cls=None)
        blitz_fn = lambda: blitzjson.dumps(data)

        json_us = benchmark(json_fn)
        blitz_us = benchmark(blitz_fn)
        speedup = json_us / blitz_us

        print(f"{name:<30} {json_us:>10.1f}µs {blitz_us:>10.1f}µs {speedup:>8.1f}x")

    if has_orjson:
        print()
        for name, data_fn in benchmarks:
            data = data_fn()
            def o_fn(d=data): return orjson.dumps(d, default=str)
            def blitz_fn(d=data): return blitzjson.dumps(d)

            o_us = benchmark(o_fn)
            blitz_us = benchmark(blitz_fn)
            speedup = o_us / blitz_us

            print(f"{name + ' (orjson)':<30} {o_us:>10.1f}µs {blitz_us:>10.1f}µs {speedup:>8.1f}x")


if __name__ == "__main__":
    run_benchmarks()
