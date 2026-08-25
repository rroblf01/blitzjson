"""blitzjson.django - Django integration for blitzjson.

Provides:
- BlitzJsonResponse: Fast JSON response using blitzjson
- BlitzJSONEncoder: JSON encoder compatible with DRF
- install()/uninstall(): Monkey-patch Python's json module
"""

import json as _stdlib_json
from blitzjson._core import dumps as _dumps, loads as _loads
from blitzjson._helpers import BlitzJSONEncoder


class BlitzJsonResponse:
    """Drop-in replacement for Django's JsonResponse.

    Uses blitzjson for serialization, which handles Django types natively:
    datetime, UUID, Decimal, QuerySet, Model, etc.

    Usage:
        from blitzjson.django import BlitzJsonResponse

        def my_view(request):
            return BlitzJsonResponse({"data": my_queryset})
    """

    def __init__(self, data, encoder=None, safe=True, json_dumps_params=None, **kwargs):
        from django.http import HttpResponse

        if safe and not isinstance(data, dict):
            raise TypeError(
                "In order to allow non-dict objects to be serialized to JSON, "
                "the `safe` argument must be set to `False`."
            )
        kwargs.setdefault("content_type", "application/json")
        data = _dumps(data, **(json_dumps_params or {}))
        self._http_response = HttpResponse(content=data, **kwargs)

    def __getattr__(self, name):
        return getattr(self._http_response, name)


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
