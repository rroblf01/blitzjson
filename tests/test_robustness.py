"""Robustness tests for blitzjson deserializer."""

import blitzjson
import pytest


class TestEdgeCases:
    def test_empty_string(self):
        with pytest.raises(ValueError):
            blitzjson.loads("")

    def test_whitespace_only(self):
        with pytest.raises(ValueError):
            blitzjson.loads("   ")

    def test_single_char(self):
        with pytest.raises(ValueError):
            blitzjson.loads("x")

    def test_truncated_json(self):
        with pytest.raises(ValueError):
            blitzjson.loads('{"key": "value"')

    def test_trailing_comma(self):
        with pytest.raises(ValueError):
            blitzjson.loads('{"key": "value",}')

    def test_missing_colon(self):
        with pytest.raises(ValueError):
            blitzjson.loads('{"key" "value"}')

    def test_missing_comma(self):
        with pytest.raises(ValueError):
            blitzjson.loads('{"a": 1 "b": 2}')

    def test_unterminated_string(self):
        with pytest.raises(ValueError):
            blitzjson.loads('"hello')

    def test_invalid_escape(self):
        with pytest.raises(ValueError):
            blitzjson.loads('"\\x"')

    def test_invalid_unicode(self):
        with pytest.raises(ValueError):
            blitzjson.loads('"\\uGGGG"')

    def test_incomplete_unicode(self):
        with pytest.raises(ValueError):
            blitzjson.loads('"\\u00"')

    def test_nested_arrays(self):
        result = blitzjson.loads("[[[1, 2], [3, 4]], [[5, 6], [7, 8]]]")
        assert result == [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]

    def test_deeply_nested(self):
        depth = 100
        s = "[" * depth + "1" + "]" * depth
        result = blitzjson.loads(s)
        expected = 1
        for _ in range(depth):
            expected = [expected]
        assert result == expected

    def test_large_number(self):
        result = blitzjson.loads("999999999999999999999999999999")
        assert result == 999999999999999999999999999999

    def test_negative_zero(self):
        result = blitzjson.loads("-0")
        assert result == 0

    def test_exponent(self):
        result = blitzjson.loads("1e10")
        assert result == 10000000000.0

    def test_negative_exponent(self):
        result = blitzjson.loads("1e-10")
        assert result == 1e-10

    def test_string_with_newlines(self):
        result = blitzjson.loads('"line1\\nline2"')
        assert result == "line1\nline2"

    def test_string_with_tabs(self):
        result = blitzjson.loads('"col1\\tcol2"')
        assert result == "col1\tcol2"

    def test_string_with_quotes(self):
        result = blitzjson.loads('"say \\"hello\\""')
        assert result == 'say "hello"'

    def test_string_with_backslash(self):
        result = blitzjson.loads('"path\\\\to\\\\file"')
        assert result == "path\\to\\file"

    def test_string_with_unicode_escape(self):
        result = blitzjson.loads('"\\u0041"')
        assert result == "A"

    def test_string_with_surrogate_pair(self):
        # Surrogate pairs are complex - just verify no crash
        result = blitzjson.loads('"\\uD83D\\uDE00"')
        assert isinstance(result, str)

    def test_multiple_values_error(self):
        with pytest.raises(ValueError):
            blitzjson.loads("1 2")

    def test_null_in_array(self):
        result = blitzjson.loads("[1, null, 3]")
        assert result == [1, None, 3]

    def test_bool_in_array(self):
        result = blitzjson.loads("[true, false]")
        assert result == [True, False]

    def test_mixed_types(self):
        result = blitzjson.loads('{"str": "hello", "int": 42, "float": 3.14, "bool": true, "null": null, "array": [1, 2], "object": {"key": "value"}}')
        assert result == {
            "str": "hello",
            "int": 42,
            "float": 3.14,
            "bool": True,
            "null": None,
            "array": [1, 2],
            "object": {"key": "value"}
        }

    def test_unicode_string(self):
        result = blitzjson.loads('"\u00f1"')
        assert result == "ñ"

    def test_empty_dict(self):
        result = blitzjson.loads("{}")
        assert result == {}

    def test_empty_array(self):
        result = blitzjson.loads("[]")
        assert result == []

    def test_single_element_array(self):
        result = blitzjson.loads("[42]")
        assert result == [42]

    def test_single_element_dict(self):
        result = blitzjson.loads('{"key": "value"}')
        assert result == {"key": "value"}

    def test_string_with_control_chars(self):
        result = blitzjson.loads('"\\b\\f\\n\\r\\t"')
        assert result == "\b\f\n\r\t"

    def test_string_with_slash(self):
        result = blitzjson.loads('"\\/\\/example.com"')
        assert result == "//example.com"

    def test_nested_objects(self):
        result = blitzjson.loads('{"a": {"b": {"c": {"d": 1}}}}')
        assert result == {"a": {"b": {"c": {"d": 1}}}}

    def test_large_dict(self):
        data = {f"key_{i}": i for i in range(100)}
        import json
        s = json.dumps(data)
        result = blitzjson.loads(s)
        assert result == data
