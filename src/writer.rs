/// Direct JSON writer with ensure_ascii, indent, and sort_keys support.
pub struct JsonWriter {
    buf: String,
    ensure_ascii: bool,
    indent: Option<u8>,
    indent_level: usize,
    sort_keys: bool,
}

impl JsonWriter {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: String::with_capacity(capacity),
            ensure_ascii: false,
            indent: None,
            indent_level: 0,
            sort_keys: false,
        }
    }

    pub fn with_options(capacity: usize, ensure_ascii: bool, indent: Option<u8>, sort_keys: bool) -> Self {
        Self {
            buf: String::with_capacity(capacity),
            ensure_ascii,
            indent,
            indent_level: 0,
            sort_keys,
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

    pub fn ensure_ascii(&self) -> bool {
        self.ensure_ascii
    }

    pub fn sort_keys(&self) -> bool {
        self.sort_keys
    }

    pub fn is_pretty(&self) -> bool {
        self.indent.is_some()
    }

    fn write_indent(&mut self) {
        if let Some(indent) = self.indent {
            self.buf.push('\n');
            for _ in 0..self.indent_level {
                for _ in 0..indent {
                    self.buf.push(' ');
                }
            }
        }
    }

    fn write_newline_or_space(&mut self) {
        if self.indent.is_some() {
            self.buf.push('\n');
        }
    }

    /// Write a JSON string with proper escaping.
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
                    self.buf.push_str("\\u00");
                    self.buf.push(HEX_CHARS[(b >> 4) as usize]);
                    self.buf.push(HEX_CHARS[(b & 0x0F) as usize]);
                    i += 1;
                }
                b if b >= 0x80 => {
                    // Decode UTF-8 sequence to get the Unicode code point
                    let _start = i;
                    i += 1;
                    let mut cp = (b & 0x1F) as u32;
                    while i < bytes.len() && (bytes[i] & 0xC0) == 0x80 {
                        cp = (cp << 6) | ((bytes[i] & 0x3F) as u32);
                        i += 1;
                    }
                    if self.ensure_ascii {
                        // Encode as \uXXXX
                        self.buf.push_str("\\u");
                        self.buf.push(HEX_CHARS[((cp >> 12) & 0x0F) as usize]);
                        self.buf.push(HEX_CHARS[((cp >> 8) & 0x0F) as usize]);
                        self.buf.push(HEX_CHARS[((cp >> 4) & 0x0F) as usize]);
                        self.buf.push(HEX_CHARS[(cp & 0x0F) as usize]);
                    } else {
                        // Push UTF-8 directly
                        if let Some(c) = char::from_u32(cp) {
                            self.buf.push(c);
                        }
                    }
                }
                _ => {
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
            self.buf.push_str(buf.format(f));
        }
    }

    pub fn write_array_open(&mut self) {
        self.buf.push('[');
        if self.indent.is_some() {
            self.indent_level += 1;
        }
    }

    pub fn write_array_close(&mut self) {
        if self.indent.is_some() {
            self.indent_level -= 1;
            self.write_indent();
        }
        self.buf.push(']');
    }

    pub fn write_object_open(&mut self) {
        self.buf.push('{');
        if self.indent.is_some() {
            self.indent_level += 1;
        }
    }

    pub fn write_object_close(&mut self) {
        if self.indent.is_some() {
            self.indent_level -= 1;
            self.write_indent();
        }
        self.buf.push('}');
    }

    pub fn write_comma(&mut self) {
        self.buf.push(',');
        if self.indent.is_some() {
            self.write_indent();
        }
    }

    pub fn write_colon(&mut self) {
        self.buf.push(':');
        if self.indent.is_some() {
            self.buf.push(' ');
        }
    }
}

const HEX_CHARS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7',
    '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];
