/// Direct JSON writer that serializes Python objects to a JSON buffer.
/// Optimized with itoa, ryu, and fast ASCII string escaping.
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

    /// Fast string escape: scan ASCII bytes directly, handle escapes inline.
    /// For non-ASCII or control chars, fall back to per-char processing.
    #[inline]
    pub fn write_string(&mut self, s: &str) {
        self.buf.push('"');
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            match b {
                b'"' => { self.buf.push_str("\\\""); i += 1; }
                b'\\' => { self.buf.push_str("\\\\"); i += 1; }
                b'\n' => { self.buf.push_str("\\n"); i += 1; }
                b'\r' => { self.buf.push_str("\\r"); i += 1; }
                b'\t' => { self.buf.push_str("\\t"); i += 1; }
                0x08 => { self.buf.push_str("\\b"); i += 1; }
                0x0C => { self.buf.push_str("\\f"); i += 1; }
                b if b < 0x20 => {
                    // Control character: write \u00XX
                    self.buf.push_str("\\u00");
                    self.buf.push(HEX_CHARS[(b >> 4) as usize]);
                    self.buf.push(HEX_CHARS[(b & 0x0F) as usize]);
                    i += 1;
                }
                b if b >= 0x80 => {
                    // Non-ASCII: scan ahead for the full UTF-8 char and push it
                    let start = i;
                    i += 1;
                    while i < bytes.len() && (bytes[i] & 0xC0) == 0x80 {
                        i += 1;
                    }
                    // SAFETY: we've verified valid UTF-8 boundaries
                    self.buf.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..i]) });
                }
                _ => {
                    // Fast path: safe ASCII byte, push directly
                    self.buf.push(b as char);
                    i += 1;
                }
            }
        }
        self.buf.push('"');
    }

    #[inline]
    pub fn write_none(&mut self) {
        self.buf.push_str("null");
    }

    #[inline]
    pub fn write_bool(&mut self, b: bool) {
        if b {
            self.buf.push_str("true");
        } else {
            self.buf.push_str("false");
        }
    }

    #[inline]
    pub fn write_i64(&mut self, i: i64) {
        let mut buf = itoa::Buffer::new();
        self.buf.push_str(buf.format(i));
    }

    #[inline]
    pub fn write_u64(&mut self, u: u64) {
        let mut buf = itoa::Buffer::new();
        self.buf.push_str(buf.format(u));
    }

    #[inline]
    pub fn write_f64(&mut self, f: f64) {
        if f.is_nan() || f.is_infinite() {
            self.buf.push_str("null");
        } else {
            let mut buf = ryu::Buffer::new();
            let s = buf.format(f);
            // ryu always includes a decimal point or 'e' for floats, so no need to check
            self.buf.push_str(s);
        }
    }

    #[inline]
    pub fn write_array_open(&mut self) {
        self.buf.push('[');
    }

    #[inline]
    pub fn write_array_close(&mut self) {
        self.buf.push(']');
    }

    #[inline]
    pub fn write_object_open(&mut self) {
        self.buf.push('{');
    }

    #[inline]
    pub fn write_object_close(&mut self) {
        self.buf.push('}');
    }

    #[inline]
    pub fn write_comma(&mut self) {
        self.buf.push(',');
    }

    #[inline]
    pub fn write_colon(&mut self) {
        self.buf.push(':');
    }
}

const HEX_CHARS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7',
    '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];
