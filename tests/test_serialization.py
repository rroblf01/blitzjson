"""Tests for blitzjson - drop-in replacement for json module."""

import json
import math
import io
from datetime import datetime, date, time, timedelta, timezone
from uuid import UUID, uuid4
from decimal import Decimal

import blitzjson


class TestBasicTypes:
    def test_none(self):
        assert blitzjson.dumps(None) == "null"

    def test_bool_true(self):
        assert blitzjson.dumps(True) == "true"

    def test_bool_false(self):
        assert blitzjson.dumps(False) == "false"

    def test_int(self):
        assert blitzjson.dumps(42) == "42"

    def test_int_negative(self):
        assert blitzjson.dumps(-42) == "-42"

    def test_int_zero(self):
        assert blitzjson.dumps(0) == "0"

    def test_float(self):
        assert blitzjson.dumps(3.14) == "3.14"

    def test_float_zero(self):
        assert blitzjson.dumps(0.0) == "0.0"

    def test_string(self):
        assert blitzjson.dumps("hello") == '"hello"'

    def test_string_empty(self):
        assert blitzjson.dumps("") == '""'

    def test_string_unicode(self):
        # serde_json outputs UTF-8 by default (modern standard)
        result = blitzjson.dumps("ñ")
        assert result == '"ñ"' or result == '"\\u00f1"'

    def test_list(self):
        assert blitzjson.dumps([1, 2, 3]) == "[1,2,3]"

    def test_list_empty(self):
        assert blitzjson.dumps([]) == "[]"

    def test_dict(self):
        result = blitzjson.dumps({"a": 1})
        assert result == '{"a":1}'

    def test_dict_empty(self):
        assert blitzjson.dumps({}) == "{}"

    def test_nested(self):
        result = blitzjson.dumps({"a": [1, 2], "b": {"c": 3}})
        assert result == '{"a":[1,2],"b":{"c":3}}'


class TestDjangoTypes:
    def test_datetime_utc(self):
        dt = datetime(2024, 1, 15, 10, 30, 45, tzinfo=timezone.utc)
        result = blitzjson.dumps(dt)
        assert result == '"2024-01-15T10:30:45+00:00"' or result == '"2024-01-15T10:30:45Z"'

    def test_datetime_naive(self):
        dt = datetime(2024, 1, 15, 10, 30, 45)
        result = blitzjson.dumps(dt)
        assert result == '"2024-01-15T10:30:45"'

    def test_datetime_with_microseconds(self):
        dt = datetime(2024, 1, 15, 10, 30, 45, 123456)
        result = blitzjson.dumps(dt)
        assert "123456" in result

    def test_date(self):
        d = date(2024, 1, 15)
        result = blitzjson.dumps(d)
        assert result == '"2024-01-15"'

    def test_time(self):
        t = time(10, 30, 45)
        result = blitzjson.dumps(t)
        assert result == '"10:30:45"'

    def test_time_with_microseconds(self):
        t = time(10, 30, 45, 123456)
        result = blitzjson.dumps(t)
        assert "123456" in result

    def test_timedelta(self):
        td = timedelta(days=1, hours=2, minutes=3, seconds=4)
        result = blitzjson.dumps(td)
        assert result == '"P1DT2H3M4S"'

    def test_timedelta_microseconds(self):
        td = timedelta(microseconds=123456)
        result = blitzjson.dumps(td)
        assert "123456" in result

    def test_uuid(self):
        u = UUID("550e8400-e29b-41d4-a716-446655440000")
        result = blitzjson.dumps(u)
        assert result == '"550e8400-e29b-41d4-a716-446655440000"'

    def test_uuid_random(self):
        u = uuid4()
        result = blitzjson.dumps(u)
        assert str(u) in result

    def test_decimal(self):
        d = Decimal("123.456")
        result = blitzjson.dumps(d)
        assert result == '"123.456"'

    def test_decimal_large_precision(self):
        d = Decimal("123.456789012345678901234567890")
        result = blitzjson.dumps(d)
        # The value is a JSON string preserving full precision
        parsed = json.loads(result)
        assert parsed == str(d)

    def test_decimal_integer(self):
        d = Decimal("100")
        result = blitzjson.dumps(d)
        assert result == '"100"'

    def test_decimal_negative(self):
        d = Decimal("-123.456")
        result = blitzjson.dumps(d)
        assert result == '"-123.456"'


class TestSpecialTypes:
    def test_set(self):
        result = blitzjson.dumps({1, 2, 3})
        parsed = json.loads(result)
        assert sorted(parsed) == [1, 2, 3]

    def test_frozenset(self):
        result = blitzjson.dumps(frozenset({1, 2, 3}))
        parsed = json.loads(result)
        assert sorted(parsed) == [1, 2, 3]

    def test_bytes(self):
        result = blitzjson.dumps(b"hello")
        assert result == '"aGVsbG8="'

    def test_bytearray(self):
        result = blitzjson.dumps(bytearray(b"hello"))
        assert result == '"aGVsbG8="'

    def test_tuple(self):
        result = blitzjson.dumps((1, 2, 3))
        assert result == "[1,2,3]"

    def test_nested_mixed(self):
        data = {
            "str": "hello",
            "int": 42,
            "float": 3.14,
            "bool": True,
            "none": None,
            "list": [1, 2, 3],
            "dict": {"nested": "value"},
        }
        result = blitzjson.dumps(data)
        parsed = json.loads(result)
        assert parsed == data


class TestLoads:
    def test_basic(self):
        assert blitzjson.loads("null") is None

    def test_bool(self):
        assert blitzjson.loads("true") is True
        assert blitzjson.loads("false") is False

    def test_int(self):
        assert blitzjson.loads("42") == 42

    def test_float(self):
        assert blitzjson.loads("3.14") == 3.14

    def test_string(self):
        assert blitzjson.loads('"hello"') == "hello"

    def test_list(self):
        assert blitzjson.loads("[1,2,3]") == [1, 2, 3]

    def test_dict(self):
        assert blitzjson.loads('{"a":1}') == {"a": 1}

    def test_nested(self):
        result = blitzjson.loads('{"a":[1,2],"b":{"c":3}}')
        assert result == {"a": [1, 2], "b": {"c": 3}}

    def test_object_hook(self):
        def hook(d):
            return {k.upper(): v for k, v in d.items()}

        result = blitzjson.loads('{"a": 1}', object_hook=hook)
        assert result == {"A": 1}

    def test_object_pairs_hook(self):
        def hook(pairs):
            return dict(pairs)

        result = blitzjson.loads('{"a": 1}', object_pairs_hook=hook)
        assert result == {"a": 1}

    def test_parse_int(self):
        result = blitzjson.loads("42", parse_int=str)
        assert result == "42"
        assert isinstance(result, str)

    def test_parse_float(self):
        result = blitzjson.loads("3.14", parse_float=str)
        assert result == "3.14"
        assert isinstance(result, str)

    def test_loads_from_bytes(self):
        result = blitzjson.loads(b'{"a": 1}')
        assert result == {"a": 1}


class TestDumpLoad:
    def test_dump_to_file(self):
        f = io.StringIO()
        blitzjson.dump({"a": 1}, f)
        f.seek(0)
        assert f.read() == '{"a":1}'

    def test_load_from_file(self):
        f = io.StringIO('{"a": 1}')
        result = blitzjson.load(f)
        assert result == {"a": 1}

    def test_dumpb(self):
        result = blitzjson.dumpb({"a": 1})
        assert isinstance(result, bytes)
        assert result == b'{"a":1}'


class TestDumpQueryset:
    def test_dump_queryset_with_list(self):
        data = [
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"},
        ]
        result = blitzjson.dump_queryset(data)
        parsed = json.loads(result)
        assert parsed == data


class TestEdgeCases:
    def test_float_nan_error(self):
        try:
            blitzjson.dumps(float("nan"))
            assert False, "Should have raised ValueError"
        except ValueError as e:
            assert "not JSON compliant" in str(e)

    def test_float_inf_error(self):
        try:
            blitzjson.dumps(float("inf"))
            assert False, "Should have raised ValueError"
        except ValueError as e:
            assert "not JSON compliant" in str(e)

    def test_deeply_nested(self):
        data = {"level": 0}
        current = data
        for i in range(1, 100):
            current["next"] = {"level": i}
            current = current["next"]
        result = blitzjson.dumps(data)
        parsed = json.loads(result)
        assert parsed == data

    def test_large_list(self):
        data = list(range(10000))
        result = blitzjson.dumps(data)
        parsed = json.loads(result)
        assert parsed == data

    def test_string_with_special_chars(self):
        data = {"key": 'value\nwith\nnewlines\tand\ttabs'}
        result = blitzjson.dumps(data)
        parsed = json.loads(result)
        assert parsed == data
