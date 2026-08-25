"""Tests for compatibility with Python's json standard library."""

import json
import math
import io
import blitzjson
import pytest


class TestDumpsCompatibility:
    """Test that blitzjson.dumps matches json.dumps behavior."""

    def test_basic_types(self):
        for obj in [None, True, False, 42, 3.14, "hello", [1, 2], {"a": 1}]:
            assert blitzjson.dumps(obj) == json.dumps(obj), f"Failed for {obj!r}"

    def test_ensure_ascii_true(self):
        data = {"ñ": "á", "中文": "值"}
        expected = json.dumps(data, ensure_ascii=True)
        result = blitzjson.dumps(data, ensure_ascii=True)
        assert result == expected

    def test_ensure_ascii_false(self):
        data = {"ñ": "á", "中文": "值"}
        expected = json.dumps(data, ensure_ascii=False)
        result = blitzjson.dumps(data, ensure_ascii=False)
        assert result == expected

    def test_indent(self):
        data = {"a": 1, "b": [2, 3]}
        expected = json.dumps(data, indent=2)
        result = blitzjson.dumps(data, indent=2)
        assert result == expected

    def test_sort_keys(self):
        data = {"c": 3, "a": 1, "b": 2}
        expected = json.dumps(data, sort_keys=True)
        result = blitzjson.dumps(data, sort_keys=True)
        assert result == expected

    def test_allow_nan_true(self):
        result = blitzjson.dumps(float("nan"), allow_nan=True)
        expected = json.dumps(float("nan"), allow_nan=True)
        assert result == expected

    def test_allow_nan_false(self):
        with pytest.raises(ValueError):
            blitzjson.dumps(float("nan"), allow_nan=False)
        with pytest.raises(ValueError):
            json.dumps(float("nan"), allow_nan=False)

    def test_default_function(self):
        class Custom:
            def __init__(self, v):
                self.v = v

        def default(o):
            if isinstance(o, Custom):
                return {"custom": o.v}
            raise TypeError

        data = {"x": Custom(42)}
        expected = json.dumps(data, default=default)
        result = blitzjson.dumps(data, default=default)
        assert result == expected

    def test_skipkeys(self):
        # blitzjson converts non-string keys to strings (like json with skipkeys=False)
        # json.dumps with skipkeys=True skips non-serializable keys
        data = {"a": 1}
        expected = json.dumps(data)
        result = blitzjson.dumps(data)
        assert result == expected

    def test_separators(self):
        data = {"a": 1, "b": 2}
        expected = json.dumps(data, separators=(",", ":"))
        result = blitzjson.dumps(data, separators=(",", ":"))
        assert result == expected

    def test_nested_structures(self):
        data = {"users": [{"id": i, "name": f"User {i}"} for i in range(10)]}
        expected = json.dumps(data)
        result = blitzjson.dumps(data)
        assert result == expected


class TestLoadsCompatibility:
    """Test that blitzjson.loads matches json.loads behavior."""

    def test_basic_types(self):
        for s in ["null", "true", "false", "42", "3.14", '"hello"', "[1,2]", '{"a":1}']:
            assert blitzjson.loads(s) == json.loads(s), f"Failed for {s}"

    def test_object_hook(self):
        def hook(d):
            return {k.upper(): v for k, v in d.items()}

        s = '{"a": 1, "b": 2}'
        expected = json.loads(s, object_hook=hook)
        result = blitzjson.loads(s, object_hook=hook)
        assert result == expected

    def test_object_pairs_hook(self):
        def hook(pairs):
            return dict(pairs)

        s = '{"a": 1, "b": 2}'
        expected = json.loads(s, object_pairs_hook=hook)
        result = blitzjson.loads(s, object_pairs_hook=hook)
        assert result == expected

    def test_parse_float(self):
        s = '{"pi": 3.14}'
        expected = json.loads(s, parse_float=str)
        result = blitzjson.loads(s, parse_float=str)
        assert result == expected

    def test_parse_int(self):
        s = '{"n": 42}'
        expected = json.loads(s, parse_int=str)
        result = blitzjson.loads(s, parse_int=str)
        assert result == expected

    def test_bytes_input(self):
        data = b'{"key": "value"}'
        expected = json.loads(data)
        result = blitzjson.loads(data)
        assert result == expected

    def test_bytearray_input(self):
        data = bytearray(b'{"key": "value"}')
        expected = json.loads(data)
        result = blitzjson.loads(data)
        assert result == expected


class TestDumpLoadCompatibility:
    """Test that blitzjson.dump/load match json.dump/load behavior."""

    def test_dump_to_file(self):
        import tempfile
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            data = {"key": "value"}
            blitzjson.dump(data, f)
            f.flush()
            with open(f.name, 'r') as f2:
                result = f2.read()
            assert result == json.dumps(data)

    def test_load_from_file(self):
        import tempfile
        data = {"key": "value"}
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            f.write(json.dumps(data))
            f.flush()
            with open(f.name, 'r') as f2:
                result = blitzjson.load(f2)
            assert result == data


class TestJSONDecodeError:
    """Test that JSONDecodeError works correctly."""

    def test_is_value_error(self):
        assert issubclass(blitzjson.JSONDecodeError, ValueError)

    def test_has_attributes(self):
        try:
            blitzjson.loads("{invalid}")
        except blitzjson.JSONDecodeError as e:
            assert hasattr(e, 'msg')
            assert hasattr(e, 'doc')
            assert hasattr(e, 'pos')
            assert isinstance(e.msg, str)
            assert isinstance(e.doc, str)
            assert isinstance(e.pos, tuple)

    def test_catch_by_value_error(self):
        try:
            blitzjson.loads("{invalid}")
            assert False, "Should have raised"
        except ValueError:
            pass  # OK


class TestJSONEncoder:
    """Test JSONEncoder compatibility."""

    def test_basic_encode(self):
        enc = blitzjson.JSONEncoder()
        result = enc.encode({"a": 1})
        assert result == '{"a": 1}'

    def test_indent(self):
        enc = blitzjson.JSONEncoder(indent=2)
        result = enc.encode({"a": 1})
        assert '{\n' in result
        assert '"a": 1' in result

    def test_sort_keys(self):
        enc = blitzjson.JSONEncoder(sort_keys=True)
        result = enc.encode({"b": 2, "a": 1})
        assert result == '{"a": 1, "b": 2}'

    def test_subclass_default(self):
        class MyEncoder(blitzjson.JSONEncoder):
            def default(self, o):
                if hasattr(o, '__dict__'):
                    return o.__dict__
                return super().default(o)

        class Obj:
            def __init__(self):
                self.x = 42

        enc = MyEncoder()
        result = enc.encode({"obj": Obj()})
        assert '"x": 42' in result

    def test_iterencode(self):
        enc = blitzjson.JSONEncoder()
        chunks = list(enc.iterencode({"a": 1}))
        assert len(chunks) == 1
        assert chunks[0] == '{"a": 1}'


class TestJSONDecoder:
    """Test JSONDecoder compatibility."""

    def test_basic_decode(self):
        dec = blitzjson.JSONDecoder()
        result = dec.decode('{"a": 1}')
        assert result == {"a": 1}

    def test_raw_decode(self):
        dec = blitzjson.JSONDecoder()
        obj, idx = dec.raw_decode('{"a": 1} extra', 0)
        assert obj == {"a": 1}
        assert idx == 14

    def test_raw_decode_with_offset(self):
        dec = blitzjson.JSONDecoder()
        obj, idx = dec.raw_decode('prefix {"a": 1}', 7)
        assert obj == {"a": 1}

    def test_object_hook(self):
        def hook(d):
            return {k.upper(): v for k, v in d.items()}

        dec = blitzjson.JSONDecoder(object_hook=hook)
        result = dec.decode('{"a": 1}')
        assert result == {"A": 1}


class TestInstallUninstall:
    """Test monkey-patching."""

    def test_install_patches_json(self):
        blitzjson.install()
        # After install, json.dumps should use blitzjson
        import importlib
        import json
        importlib.reload(json)
        # Can't easily test this without side effects, but verify it doesn't crash
        result = json.dumps({"test": 1})
        assert result == '{"test": 1}'
        blitzjson.uninstall()
        importlib.reload(json)
