use std::fmt::Write;

/// Direct JSON writer that serializes Python objects to a JSON buffer
/// without intermediate serde_json::Value allocation.
pub struct JsonWriter {
    buf: String,
}

impl JsonWriter {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: String::with_capacity(capacity),
        }
    }

    pub fn into_string(self) -> String {
        self.buf
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf.into_bytes()
    }

    pub fn buf_mut(&mut self) -> &mut String {
        &mut self.buf
    }

    #[inline]
    fn write_str_escaped(&mut self, s: &str) {
        self.buf.push('"');
        for c in s.chars() {
            match c {
                '"' => self.buf.push_str("\\\""),
                '\\' => self.buf.push_str("\\\\"),
                '\n' => self.buf.push_str("\\n"),
                '\r' => self.buf.push_str("\\r"),
                '\t' => self.buf.push_str("\\t"),
                '\x08' => self.buf.push_str("\\b"),
                '\x0c' => self.buf.push_str("\\f"),
                c if c < '\x20' => {
                    write!(self.buf, "\\u{:04x}", c as u32).unwrap();
                }
                c if c > '\u{7f}' => {
                    // Output UTF-8 directly (modern standard)
                    self.buf.push(c);
                }
                c => self.buf.push(c),
            }
        }
        self.buf.push('"');
    }

    pub fn write_none(&mut self) {
        self.buf.push_str("null");
    }

    pub fn write_bool(&mut self, b: bool) {
        if b {
            self.buf.push_str("true");
        } else {
            self.buf.push_str("false");
        }
    }

    pub fn write_i64(&mut self, i: i64) {
        write!(self.buf, "{}", i).unwrap();
    }

    pub fn write_f64(&mut self, f: f64) {
        if f.is_nan() || f.is_infinite() {
            self.buf.push_str("null");
        } else {
            // Ensure decimal point is always present for floats
            let s = format!("{}", f);
            if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                self.buf.push_str(&format!("{}.0", s));
            } else {
                self.buf.push_str(&s);
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        self.write_str_escaped(s);
    }

    pub fn write_array_open(&mut self) {
        self.buf.push('[');
    }

    pub fn write_array_close(&mut self) {
        self.buf.push(']');
    }

    pub fn write_object_open(&mut self) {
        self.buf.push('{');
    }

    pub fn write_object_close(&mut self) {
        self.buf.push('}');
    }

    pub fn write_comma(&mut self) {
        self.buf.push(',');
    }

    pub fn write_colon(&mut self) {
        self.buf.push(':');
    }
}
