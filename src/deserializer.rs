use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::IntoPyObjectExt;

/// Direct JSON deserializer that creates Python objects without intermediate serde_json::Value.
///
/// # Safety
/// This function parses JSON and creates Python objects directly.
pub fn deserialize_direct<'py>(
    py: Python<'py>,
    s: &str,
    object_hook: Option<&Bound<'py, PyAny>>,
    object_pairs_hook: Option<&Bound<'py, PyAny>>,
    parse_float: Option<&Bound<'py, PyAny>>,
    parse_int: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let bytes = s.as_bytes();
    let mut parser = JsonParser::new(bytes);
    let result = parser.parse_value(py, object_hook, object_pairs_hook, parse_float, parse_int)?;
    Ok(result)
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    #[inline]
    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b' ' | b'\n' | b'\r' | b'\t' => self.pos += 1,
                _ => break,
            }
        }
    }

    #[inline]
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    #[inline]
    fn next_byte(&mut self) -> Option<u8> {
        let b = self.bytes.get(self.pos).copied();
        if b.is_some() { self.pos += 1; }
        b
    }

    fn parse_value<'py>(
        &mut self,
        py: Python<'py>,
        object_hook: Option<&Bound<'py, PyAny>>,
        object_pairs_hook: Option<&Bound<'py, PyAny>>,
        parse_float: Option<&Bound<'py, PyAny>>,
        parse_int: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'"') => self.parse_string(py).map(|s| s.into_any()),
            Some(b'{') => self.parse_object(py, object_hook, object_pairs_hook, parse_float, parse_int),
            Some(b'[') => self.parse_array(py, object_hook, object_pairs_hook, parse_float, parse_int),
            Some(b't') | Some(b'f') => self.parse_bool(py),
            Some(b'n') => self.parse_null(py),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(py, parse_float, parse_int),
            _ => Err(pyo3::exceptions::PyValueError::new_err("Invalid JSON")),
        }
    }

    fn parse_string<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.next_byte(); // consume opening "
        let start = self.pos;
        let mut result = String::with_capacity(64);

        loop {
            match self.next_byte() {
                Some(b'"') => break,
                Some(b'\\') => {
                    // Copy the unescaped portion
                    if start < self.pos - 1 {
                        let raw = unsafe {
                            std::str::from_utf8_unchecked(&self.bytes[start..self.pos - 1])
                        };
                        result.push_str(raw);
                    }
                    match self.next_byte() {
                        Some(b'"') => result.push('"'),
                        Some(b'\\') => result.push('\\'),
                        Some(b'/') => result.push('/'),
                        Some(b'n') => result.push('\n'),
                        Some(b'r') => result.push('\r'),
                        Some(b't') => result.push('\t'),
                        Some(b'b') => result.push('\x08'),
                        Some(b'f') => result.push('\x0c'),
                        Some(b'u') => {
                            let hex = self.parse_hex4()?;
                            if let Some(c) = char::from_u32(hex) {
                                result.push(c);
                            }
                        }
                        _ => return Err(pyo3::exceptions::PyValueError::new_err("Invalid escape")),
                    }
                    // Reset start to after the escape
                    // SAFETY: self.pos is always valid here
                }
                None => return Err(pyo3::exceptions::PyValueError::new_err("Unterminated string")),
                _ => {}
            }
        }

        // Copy remaining unescaped portion
        if start < self.pos - 1 {
            let raw = unsafe {
                std::str::from_utf8_unchecked(&self.bytes[start..self.pos - 1])
            };
            result.push_str(raw);
        }

        Ok(result.into_pyobject(py)?.into_any())
    }

    fn parse_hex4(&mut self) -> PyResult<u32> {
        let mut value: u32 = 0;
        for _ in 0..4 {
            let b = self.next_byte().ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("Invalid Unicode escape")
            })?;
            value = (value << 4) | match b {
                b'0'..=b'9' => (b - b'0') as u32,
                b'a'..=b'f' => (b - b'a' + 10) as u32,
                b'A'..=b'F' => (b - b'A' + 10) as u32,
                _ => return Err(pyo3::exceptions::PyValueError::new_err("Invalid hex digit")),
            };
        }
        Ok(value)
    }

    fn parse_object<'py>(
        &mut self,
        py: Python<'py>,
        object_hook: Option<&Bound<'py, PyAny>>,
        object_pairs_hook: Option<&Bound<'py, PyAny>>,
        parse_float: Option<&Bound<'py, PyAny>>,
        parse_int: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.next_byte(); // consume {
        let dict = PyDict::new(py);
        self.skip_whitespace();

        if self.peek() == Some(b'}') {
            self.next_byte();
        } else {
            loop {
                self.skip_whitespace();
                let key = self.parse_string(py)?;
                self.skip_whitespace();
                self.next_byte(); // consume :
                self.skip_whitespace();
                let value = self.parse_value(py, object_hook, object_pairs_hook, parse_float, parse_int)?;
                dict.set_item(&key, &value)?;

                self.skip_whitespace();
                match self.next_byte() {
                    Some(b'}') => break,
                    Some(b',') => continue,
                    _ => return Err(pyo3::exceptions::PyValueError::new_err("Expected ',' or '}'")),
                }
            }
        }

        if let Some(hook) = object_pairs_hook {
            let pairs = PyList::empty(py);
            for (k, v) in dict.iter() {
                let pair = PyList::empty(py);
                pair.append(k)?;
                pair.append(v)?;
                pairs.append(pair)?;
            }
            return hook.call1((pairs,));
        }

        if let Some(hook) = object_hook {
            return hook.call1((&dict,));
        }

        Ok(dict.into_any())
    }

    fn parse_array<'py>(
        &mut self,
        py: Python<'py>,
        object_hook: Option<&Bound<'py, PyAny>>,
        object_pairs_hook: Option<&Bound<'py, PyAny>>,
        parse_float: Option<&Bound<'py, PyAny>>,
        parse_int: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.next_byte(); // consume [
        let list = PyList::empty(py);
        self.skip_whitespace();

        if self.peek() == Some(b']') {
            self.next_byte();
        } else {
            loop {
                self.skip_whitespace();
                let value = self.parse_value(py, object_hook, object_pairs_hook, parse_float, parse_int)?;
                list.append(value)?;

                self.skip_whitespace();
                match self.next_byte() {
                    Some(b']') => break,
                    Some(b',') => continue,
                    _ => return Err(pyo3::exceptions::PyValueError::new_err("Expected ',' or ']'")),
                }
            }
        }

        Ok(list.into_any())
    }

    fn parse_bool<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        if self.bytes[self.pos..].starts_with(b"true") {
            self.pos += 4;
            let obj: Bound<'py, PyAny> = true.into_py_any(py)?.into_bound(py);
            Ok(obj)
        } else if self.bytes[self.pos..].starts_with(b"false") {
            self.pos += 5;
            let obj: Bound<'py, PyAny> = false.into_py_any(py)?.into_bound(py);
            Ok(obj)
        } else {
            Err(pyo3::exceptions::PyValueError::new_err("Invalid boolean"))
        }
    }

    fn parse_null<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        if self.bytes[self.pos..].starts_with(b"null") {
            self.pos += 4;
            Ok(py.None().into_bound(py).into_any())
        } else {
            Err(pyo3::exceptions::PyValueError::new_err("Invalid null"))
        }
    }

    fn parse_number<'py>(
        &mut self,
        py: Python<'py>,
        parse_float: Option<&Bound<'py, PyAny>>,
        parse_int: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let start = self.pos;

        // Consume the number
        if self.peek() == Some(b'-') { self.pos += 1; }
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let has_decimal = self.peek() == Some(b'.');
        if has_decimal {
            self.pos += 1;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        let has_exponent = matches!(self.peek(), Some(b'e') | Some(b'E'));
        if has_exponent {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) { self.pos += 1; }
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        let num_str = unsafe { std::str::from_utf8_unchecked(&self.bytes[start..self.pos]) };

        if has_decimal || has_exponent {
            // Float
            if let Some(pf) = parse_float {
                return pf.call1((num_str,));
            }
            let val: f64 = num_str.parse().map_err(|_| {
                pyo3::exceptions::PyValueError::new_err("Invalid float")
            })?;
            Ok(val.into_pyobject(py)?.into_any())
        } else {
            // Integer
            if let Some(pi) = parse_int {
                return pi.call1((num_str,));
            }
            if let Ok(i) = num_str.parse::<i64>() {
                Ok(i.into_pyobject(py)?.into_any())
            } else if let Ok(u) = num_str.parse::<u64>() {
                Ok(u.into_pyobject(py)?.into_any())
            } else {
                // BigInt: fallback to Python int
                let py_int = py.import("builtins")?.call_method1("int", (num_str,))?;
                Ok(py_int.into_any())
            }
        }
    }
}
