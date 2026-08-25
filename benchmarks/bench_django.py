"""Django-specific benchmarks with realistic data."""
import json
import timeit
import statistics
from datetime import datetime, date, time, timedelta, timezone
from uuid import UUID, uuid4
from decimal import Decimal


def benchmark(func, number=1000, rounds=3):
    times = timeit.repeat(func, number=number, repeat=rounds)
    return statistics.median([t / number * 1_000_000 for t in times])


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


def django_order_detail():
    return {
        'order_id': '550e8400-e29b-41d4-a716-446655440000',
        'customer': {'id': 1, 'name': 'Alice', 'email': 'alice@example.com'},
        'items': [
            {'product_id': i, 'name': f'Item {i}', 'quantity': i + 1,
             'unit_price': float(f'{(i+1)*19.99:.2f}'), 'total': float(f'{(i+1)*19.99:.2f}')}
            for i in range(5)
        ],
        'subtotal': float('299.95'), 'tax': float('62.99'), 'total': float('362.94'),
        'status': 'completed', 'created_at': '2024-01-15T10:30:45Z', 'updated_at': '2024-01-15T10:35:00Z'
    }


def django_api_response():
    return {
        'status': 'success', 'data': {
            'products': [{'id': i, 'name': f'Product {i}', 'price': 29.99, 'sku': f'SKU-{i:06d}',
                          'in_stock': i % 3 != 0, 'created': '2024-01-15T10:30:45Z'} for i in range(20)],
            'categories': ['Electronics', 'Books', 'Clothing'], 'total_count': 20
        },
        'meta': {'request_id': '550e8400-e29b-41d4-a716-446655440000', 'version': '1.0'}
    }


def django_large_dataset():
    return {
        'results': [{'id': i, 'field1': f'value_{i}', 'field2': i * 1.5,
                     'field3': True, 'field4': f'data_{i}@example.com',
                     'field5': '2024-01-15T10:30:45Z'} for i in range(200)],
        'count': 200, 'next': None, 'previous': None
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
        ("Django API Response", django_api_response),
        ("Django Large Dataset (200)", django_large_dataset),
    ]

    print("=" * 75)
    print("DJANGO-SPECIFIC BENCHMARKS")
    print("=" * 75)
    print(f"{'Benchmark':<30} {'json+DJE':>12} {'blitzjson':>12} {'Speedup':>10}")
    print("-" * 75)

    for name, data_fn in benchmarks:
        data = data_fn()
        json_fn = lambda: json.dumps(data)
        blitz_fn = lambda: blitzjson.dumps(data)
        json_us = benchmark(json_fn)
        blitz_us = benchmark(blitz_fn)
        print(f"{name:<30} {json_us:>10.1f}µs {blitz_us:>10.1f}µs {json_us/blitz_us:>8.1f}x")

    if has_orjson:
        print()
        print(f"{'Benchmark':<30} {'orjson':>12} {'blitzjson':>12} {'Speedup':>10}")
        print("-" * 75)
        for name, data_fn in benchmarks:
            data = data_fn()
            o_fn = lambda: orjson.dumps(data)
            blitz_fn = lambda: blitzjson.dumps(data)
            o_us = benchmark(o_fn)
            blitz_us = benchmark(blitz_fn)
            print(f"{name:<30} {o_us:>10.1f}µs {blitz_us:>10.1f}µs {o_us/blitz_us:>8.1f}x")


if __name__ == "__main__":
    run_benchmarks()
