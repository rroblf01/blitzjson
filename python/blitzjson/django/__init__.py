"""blitzjson.django - Django integration for blitzjson.

Provides:
- BlitzJsonResponse: Fast JSON response using blitzjson
- BlitzJSONEncoder: JSON encoder compatible with DRF
- install(): Monkey-patch Python's json module with blitzjson
"""

import json as _stdlib_json
from django.http import HttpResponse
from blitzjson._core import dumps as _dumps, loads as _loads


class BlitzJsonResponse(HttpResponse):
    """Drop-in replacement for Django's JsonResponse.

    Uses blitzjson for serialization, which handles Django types natively:
    datetime, UUID, Decimal, QuerySet, Model, etc.

    Usage:
        from blitzjson.django import BlitzJsonResponse

        def my_view(request):
            return BlitzJsonResponse({"data": my_queryset})
    """

    def __init__(self, data, encoder=None, safe=True, json_dumps_params=None, **kwargs):
        if safe and not isinstance(data, dict):
            raise TypeError(
                "In order to allow non-dict objects to be serialized to JSON, "
                "the `safe` argument must be set to `False`."
            )
        kwargs.setdefault("content_type", "application/json")
        data = _dumps(data, **(json_dumps_params or {}))
        super().__init__(content=data, **kwargs)


class BlitzJSONEncoder(_stdlib_json.JSONEncoder):
    """JSON encoder that uses blitzjson for serialization.

    Compatible with Django REST Framework and any code that uses
    json.JSONEncoder as a base class.

    Usage:
        import json
        from blitzjson.django import BlitzJSONEncoder

        json.dumps(data, cls=BlitzJSONEncoder)
    """

    def default(self, o):
        # Delegate to blitzjson's native type handling
        # This is called for types that json.dumps can't handle natively
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


def install():
    """Monkey-patch Python's json module with blitzjson.

    After calling this, `import json` will use blitzjson for all
    serialization/deserialization operations.

    Usage:
        # In settings.py or at app startup
        import blitzjson
        blitzjson.install()

    WARNING: This modifies the global json module. Use with caution.
    """
    _stdlib_json.dumps = _dumps
    _stdlib_json.loads = _loads
    _stdlib_json.dump = lambda obj, fp, **kwargs: fp.write(_dumps(obj, **kwargs))
    _stdlib_json.load = lambda fp, **kwargs: _loads(fp.read(), **kwargs)
    _stdlib_json.JSONEncoder = BlitzJSONEncoder


def uninstall():
    """Revert monkey-patching done by install().

    Restores the original json module functions.
    """
    import importlib
    importlib.reload(_stdlib_json)
