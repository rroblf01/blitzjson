/// Direct JSON writer with ensure_ascii, indent, sort_keys, and separators support.
/// Optimized with fast ASCII string path and inline hints.
pub struct JsonWriter {
    buf: String,
    ensure_ascii: bool,
    indent: Option<u8>,
    indent_level: usize,
    sort_keys: bool,
    item_sep: &'static str,
    key_sep: &'static str,
}

impl JsonWriter {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: String::with_capacity(capacity),
            ensure_ascii: false,
            indent: None,
            indent_level: 0,
            sort_keys: false,
            item_sep: ", ",
            key_sep: ": ",
        }
    }

    pub fn with_options(capacity: usize, ensure_ascii: bool, indent: Option<u8>, sort_keys: bool) -> Self {
        let (item_sep, key_sep) = if indent.is_some() { (",", ": ") } else { (", ", ": ") };
        Self { buf: String::with_capacity(capacity), ensure_ascii, indent, indent_level: 0, sort_keys, item_sep, key_sep }
    }

    pub fn with_separators(capacity: usize, ensure_ascii: bool, indent: Option<u8>,
                           sort_keys: bool, item_sep: &'static str, key_sep: &'static str) -> Self {
        Self { buf: String::with_capacity(capacity), ensure_ascii, indent, indent_level: 0, sort_keys, item_sep, key_sep }
    }

    pub fn into_string(self) -> String { self.buf }
    pub fn into_bytes(self) -> Vec<u8> { self.buf.into_bytes() }
    pub fn buf_mut(&mut self) -> &mut String { &mut self.buf }
    pub fn ensure_ascii(&self) -> bool { self.ensure_ascii }
    pub fn sort_keys(&self) -> bool { self.sort_keys }

    #[inline(always)]
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

    /// Fast JSON string escape with ASCII fast path.
    #[inline(always)]
    pub fn write_string(&mut self, s: &str) {
        // Fast path: scan for special chars in a single pass
        let bytes = s.as_bytes();
        let mut needs_escape = false;
        for &b in bytes {
            if b < 0x20 || b == b'"' || b == b'\\' || b >= 0x80 {
                needs_escape = true;
                break;
            }
        }

        self.buf.push('"');
        if !needs_escape {
            // Fast path: no escaping needed, copy entire string
            self.buf.push_str(s);
        } else {
            self.escape_slow(bytes);
        }
        self.buf.push('"');
    }

    #[cold]
    #[inline(never)]
    fn escape_slow(&mut self, bytes: &[u8]) {
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
                    i += 1;
                    let mut cp = (b & 0x1F) as u32;
                    while i < bytes.len() && (bytes[i] & 0xC0) == 0x80 {
                        cp = (cp << 6) | ((bytes[i] & 0x3F) as u32);
                        i += 1;
                    }
                    if self.ensure_ascii {
                        self.buf.push_str("\\u");
                        self.buf.push(HEX_CHARS[((cp >> 12) & 0x0F) as usize]);
                        self.buf.push(HEX_CHARS[((cp >> 8) & 0x0F) as usize]);
                        self.buf.push(HEX_CHARS[((cp >> 4) & 0x0F) as usize]);
                        self.buf.push(HEX_CHARS[(cp & 0x0F) as usize]);
                    } else if let Some(c) = char::from_u32(cp) {
                        self.buf.push(c);
                    }
                }
                _ => { self.buf.push(b as char); i += 1; }
            }
        }
    }

    #[inline(always)]
    pub fn write_none(&mut self) { self.buf.push_str("null"); }

    #[inline(always)]
    pub fn write_raw(&mut self, s: &str) { self.buf.push_str(s); }

    #[inline(always)]
    pub fn write_bool(&mut self, b: bool) {
        if b { self.buf.push_str("true"); } else { self.buf.push_str("false"); }
    }

    #[inline(always)]
    pub fn write_i64(&mut self, i: i64) {
        let mut buf = itoa::Buffer::new();
        self.buf.push_str(buf.format(i));
    }

    #[inline(always)]
    pub fn write_u64(&mut self, u: u64) {
        let mut buf = itoa::Buffer::new();
        self.buf.push_str(buf.format(u));
    }

    #[inline(always)]
    pub fn write_f64(&mut self, f: f64) {
        let mut buf = ryu::Buffer::new();
        self.buf.push_str(buf.format(f));
    }

    #[inline(always)]
    pub fn write_array_open(&mut self) {
        self.buf.push('[');
        if self.indent.is_some() { self.indent_level += 1; self.write_indent(); }
    }

    #[inline(always)]
    pub fn write_array_close(&mut self) {
        if self.indent.is_some() { self.indent_level -= 1; self.write_indent(); }
        self.buf.push(']');
    }

    #[inline(always)]
    pub fn write_object_open(&mut self) {
        self.buf.push('{');
        if self.indent.is_some() { self.indent_level += 1; self.write_indent(); }
    }

    #[inline(always)]
    pub fn write_object_close(&mut self) {
        if self.indent.is_some() { self.indent_level -= 1; self.write_indent(); }
        self.buf.push('}');
    }

    #[inline(always)]
    pub fn write_comma(&mut self) {
        self.buf.push_str(self.item_sep);
        if self.indent.is_some() { self.write_indent(); }
    }

    #[inline(always)]
    pub fn write_colon(&mut self) { self.buf.push_str(self.key_sep); }
}

const HEX_CHARS: [char; 16] = ['0','1','2','3','4','5','6','7','8','9','a','b','c','d','e','f'];
