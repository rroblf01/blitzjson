use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::IntoPyObjectExt;

pub fn deserialize_direct<'py>(
    py: Python<'py>,
    s: &str,
    object_hook: Option<&Bound<'py, PyAny>>,
    object_pairs_hook: Option<&Bound<'py, PyAny>>,
    parse_float: Option<&Bound<'py, PyAny>>,
    parse_int: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let bytes = s.as_bytes();
    let mut parser = JsonParser::new(bytes, s);
    parser.parse_value(py, object_hook, object_pairs_hook, parse_float, parse_int)
}

pub fn deserialize_strict<'py>(
    py: Python<'py>,
    s: &str,
    object_hook: Option<&Bound<'py, PyAny>>,
    object_pairs_hook: Option<&Bound<'py, PyAny>>,
    parse_float: Option<&Bound<'py, PyAny>>,
    parse_int: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let bytes = s.as_bytes();
    let mut parser = JsonParser::new(bytes, s);
    let result = parser.parse_value(py, object_hook, object_pairs_hook, parse_float, parse_int)?;
    parser.skip_whitespace();
    if parser.pos < parser.bytes.len() {
        return Err(parser.error("Extra data after JSON value"));
    }
    Ok(result)
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
    doc: &'a str,
}

impl<'a> JsonParser<'a> {
    fn new(bytes: &'a [u8], doc: &'a str) -> Self {
        Self { bytes, pos: 0, doc }
    }

    #[inline(always)]
    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b' ' | b'\n' | b'\r' | b'\t' => self.pos += 1,
                _ => break,
            }
        }
    }

    #[inline(always)]
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    #[inline(always)]
    fn next_byte(&mut self) -> Option<u8> {
        let b = self.bytes.get(self.pos).copied();
        if b.is_some() { self.pos += 1; }
        b
    }

    fn error(&self, msg: &str) -> PyErr {
        let row = self.bytes[..self.pos].iter().filter(|&&b| b == b'\n').count() + 1;
        let last_newline = self.bytes[..self.pos].iter().rposition(|&b| b == b'\n');
        let col = match last_newline {
            Some(pos) => self.pos - pos,
            None => self.pos + 1,
        };
        pyo3::exceptions::PyValueError::new_err(format!("{} at row {}, column {}", msg, row, col))
    }

    #[inline(always)]
    fn parse_value<'py>(
        &mut self,
        py: Python<'py>,
        oh: Option<&Bound<'py, PyAny>>,
        oph: Option<&Bound<'py, PyAny>>,
        pf: Option<&Bound<'py, PyAny>>,
        pi: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'"') => self.parse_string(py),
            Some(b'{') => self.parse_object(py, oh, oph, pf, pi),
            Some(b'[') => self.parse_array(py, oh, oph, pf, pi),
            Some(b't') | Some(b'f') => self.parse_bool(py),
            Some(b'n') => self.parse_null(py),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(py, pf, pi),
            _ => Err(self.error("Invalid JSON value")),
        }
    }

    #[inline]
    fn parse_string<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.next_byte();
        let start = self.pos;
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b'"' => {
                    let s = unsafe { std::str::from_utf8_unchecked(&self.bytes[start..self.pos]) };
                    self.pos += 1;
                    return Ok(s.into_pyobject(py)?.into_any());
                }
                b'\\' => break,
                _ => self.pos += 1,
            }
        }
        let mut result = String::with_capacity(64);
        self.pos = start;
        loop {
            match self.next_byte() {
                Some(b'"') => break,
                Some(b'\\') => {
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
                            if let Some(c) = char::from_u32(hex) { result.push(c); }
                        }
                        _ => return Err(self.error("Invalid escape sequence")),
                    }
                }
                None => return Err(self.error("Unterminated string")),
                Some(c) => result.push(c as char),
            }
        }
        Ok(result.into_pyobject(py)?.into_any())
    }

    fn parse_hex4(&mut self) -> PyResult<u32> {
        let mut value: u32 = 0;
        for _ in 0..4 {
            let b = self.next_byte().ok_or_else(|| self.error("Invalid Unicode escape"))?;
            value = (value << 4) | match b {
                b'0'..=b'9' => (b - b'0') as u32,
                b'a'..=b'f' => (b - b'a' + 10) as u32,
                b'A'..=b'F' => (b - b'A' + 10) as u32,
                _ => return Err(self.error("Invalid hex digit")),
            };
        }
        Ok(value)
    }

    #[inline]
    fn parse_object<'py>(
        &mut self,
        py: Python<'py>,
        oh: Option<&Bound<'py, PyAny>>,
        oph: Option<&Bound<'py, PyAny>>,
        pf: Option<&Bound<'py, PyAny>>,
        pi: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.next_byte();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.next_byte();
            let dict = PyDict::new(py);
            if let Some(hook) = oph { return hook.call1((PyList::empty(py),)); }
            if let Some(hook) = oh { return hook.call1((&dict,)); }
            return Ok(dict.into_any());
        }
        let dict = PyDict::new(py);
        loop {
            self.skip_whitespace();
            let key = self.parse_string(py)?;
            self.skip_whitespace();
            match self.next_byte() { Some(b':') => {}, _ => return Err(self.error("Expected ':'")) }
            self.skip_whitespace();
            let value = self.parse_value(py, oh, oph, pf, pi)?;
            dict.set_item(&key, &value)?;
            self.skip_whitespace();
            match self.next_byte() {
                Some(b'}') => break,
                Some(b',') => continue,
                _ => return Err(self.error("Expected ',' or '}'")),
            }
        }
        if let Some(hook) = oph {
            let pairs = PyList::empty(py);
            for (k, v) in dict.iter() {
                let pair = PyList::empty(py);
                pair.append(k)?;
                pair.append(v)?;
                pairs.append(pair)?;
            }
            return hook.call1((pairs,));
        }
        if let Some(hook) = oh { return hook.call1((&dict,)); }
        Ok(dict.into_any())
    }

    #[inline]
    fn parse_array<'py>(
        &mut self,
        py: Python<'py>,
        oh: Option<&Bound<'py, PyAny>>,
        oph: Option<&Bound<'py, PyAny>>,
        pf: Option<&Bound<'py, PyAny>>,
        pi: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.next_byte();
        let list = PyList::empty(py);
        self.skip_whitespace();
        if self.peek() == Some(b']') { self.next_byte(); return Ok(list.into_any()); }
        loop {
            self.skip_whitespace();
            let value = self.parse_value(py, oh, oph, pf, pi)?;
            list.append(value)?;
            self.skip_whitespace();
            match self.next_byte() {
                Some(b']') => break,
                Some(b',') => continue,
                _ => return Err(self.error("Expected ',' or ']'")),
            }
        }
        Ok(list.into_any())
    }

    #[inline]
    fn parse_bool<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        if self.bytes[self.pos..].starts_with(b"true") {
            self.pos += 4;
            Ok(true.into_py_any(py)?.into_bound(py))
        } else if self.bytes[self.pos..].starts_with(b"false") {
            self.pos += 5;
            Ok(false.into_py_any(py)?.into_bound(py))
        } else {
            Err(self.error("Invalid boolean"))
        }
    }

    #[inline]
    fn parse_null<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        if self.bytes[self.pos..].starts_with(b"null") {
            self.pos += 4;
            Ok(py.None().into_bound(py).into_any())
        } else {
            Err(self.error("Invalid null"))
        }
    }

    #[inline]
    fn parse_number<'py>(
        &mut self,
        py: Python<'py>,
        pf: Option<&Bound<'py, PyAny>>,
        pi: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let start = self.pos;
        if self.peek() == Some(b'-') { self.pos += 1; }
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() { self.pos += 1; }
        let has_decimal = self.peek() == Some(b'.');
        if has_decimal {
            self.pos += 1;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() { self.pos += 1; }
        }
        let has_exponent = matches!(self.peek(), Some(b'e') | Some(b'E'));
        if has_exponent {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) { self.pos += 1; }
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() { self.pos += 1; }
        }
        let num_str = unsafe { std::str::from_utf8_unchecked(&self.bytes[start..self.pos]) };
        if has_decimal || has_exponent {
            if let Some(pf) = pf { return pf.call1((num_str,)); }
            let val: f64 = num_str.parse().map_err(|_| self.error("Invalid float"))?;
            Ok(val.into_pyobject(py)?.into_any())
        } else {
            if let Some(pi) = pi { return pi.call1((num_str,)); }
            if let Ok(i) = num_str.parse::<i64>() { Ok(i.into_pyobject(py)?.into_any()) }
            else if let Ok(u) = num_str.parse::<u64>() { Ok(u.into_pyobject(py)?.into_any()) }
            else {
                let py_int = py.import("builtins")?.call_method1("int", (num_str,))?;
                Ok(py_int.into_any())
            }
        }
    }
}
