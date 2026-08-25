use std::ffi::CStr;
use std::fmt::Write as FmtWrite;
use pyo3::prelude::*;
use pyo3::ffi::*;
use std::os::raw::{c_char, c_int};
use crate::writer::JsonWriter;

const MAX_DEPTH: usize = 128;

// ── Cached type pointers ──────────────────────────────────────────

struct CachedTypes {
    promise: *mut PyObject,
    model: *mut PyObject,
    queryset: *mut PyObject,
    enum_type: *mut PyObject,
    datetime_type: *mut PyObject,
    date_type: *mut PyObject,
    time_type: *mut PyObject,
    timedelta_type: *mut PyObject,
    uuid_type: *mut PyObject,
    decimal_type: *mut PyObject,
}
unsafe impl Send for CachedTypes {}
unsafe impl Sync for CachedTypes {}
static mut CACHED_TYPES: Option<CachedTypes> = None;

unsafe fn get_cached_types(py: Python<'_>) -> Option<&'static CachedTypes> {
    if CACHED_TYPES.is_some() { return CACHED_TYPES.as_ref(); }
    let promise = py.import("django.utils.functional").ok().and_then(|m| m.getattr("Promise").ok()).map(|o| o.as_ptr());
    let model = py.import("django.db.models").ok().and_then(|m| m.getattr("Model").ok()).map(|o| o.as_ptr());
    let queryset = py.import("django.db.models.query").ok().and_then(|m| m.getattr("QuerySet").ok()).map(|o| o.as_ptr());
    let enum_type = py.import("enum").ok().and_then(|m| m.getattr("Enum").ok()).map(|o| o.as_ptr());
    let datetime_type = py.import("datetime").ok().and_then(|m| m.getattr("datetime").ok()).map(|o| o.as_ptr());
    let date_type = py.import("datetime").ok().and_then(|m| m.getattr("date").ok()).map(|o| o.as_ptr());
    let time_type = py.import("datetime").ok().and_then(|m| m.getattr("time").ok()).map(|o| o.as_ptr());
    let timedelta_type = py.import("datetime").ok().and_then(|m| m.getattr("timedelta").ok()).map(|o| o.as_ptr());
    let uuid_type = py.import("uuid").ok().and_then(|m| m.getattr("UUID").ok()).map(|o| o.as_ptr());
    let decimal_type = py.import("decimal").ok().and_then(|m| m.getattr("Decimal").ok()).map(|o| o.as_ptr());
    CACHED_TYPES = Some(CachedTypes {
        promise: promise.unwrap_or(std::ptr::null_mut()),
        model: model.unwrap_or(std::ptr::null_mut()),
        queryset: queryset.unwrap_or(std::ptr::null_mut()),
        enum_type: enum_type.unwrap_or(std::ptr::null_mut()),
        datetime_type: datetime_type.unwrap_or(std::ptr::null_mut()),
        date_type: date_type.unwrap_or(std::ptr::null_mut()),
        time_type: time_type.unwrap_or(std::ptr::null_mut()),
        timedelta_type: timedelta_type.unwrap_or(std::ptr::null_mut()),
        uuid_type: uuid_type.unwrap_or(std::ptr::null_mut()),
        decimal_type: decimal_type.unwrap_or(std::ptr::null_mut()),
    });
    CACHED_TYPES.as_ref()
}

// ── Main serializer ───────────────────────────────────────────────

#[inline(always)]
pub unsafe fn ffi_serialize(
    py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter,
    depth: usize, default: *mut PyObject, allow_nan: bool,
) -> Result<(), PyErr> {
    if depth >= MAX_DEPTH {
        return Err(pyo3::exceptions::PyValueError::new_err("Maximum recursion depth exceeded"));
    }
    if obj.is_null() || Py_IsNone(obj) != 0 { w.write_none(); return Ok(()); }
    if PyBool_Check(obj) != 0 { w.write_bool(obj == Py_True()); return Ok(()); }
    if PyLong_Check(obj) != 0 {
        let mut overflow: c_int = 0;
        let val = PyLong_AsLongLongAndOverflow(obj, &mut overflow);
        if overflow == 0 { w.write_i64(val as i64); }
        else { write_pyrepr(w, obj); }
        return Ok(());
    }
    if PyFloat_Check(obj) != 0 {
        let val = PyFloat_AS_DOUBLE(obj);
        if val.is_nan() {
            if allow_nan { w.write_raw("NaN"); }
            else { return Err(pyo3::exceptions::PyValueError::new_err("Out of range float values are not JSON compliant")); }
        } else if val.is_infinite() {
            if allow_nan {
                if val > 0.0 { w.write_raw("Infinity"); } else { w.write_raw("-Infinity"); }
            } else {
                return Err(pyo3::exceptions::PyValueError::new_err("Out of range float values are not JSON compliant"));
            }
        } else { w.write_f64(val); }
        return Ok(());
    }
    if PyUnicode_Check(obj) != 0 {
        let mut size: isize = 0;
        let ptr = PyUnicode_AsUTF8AndSize(obj, &mut size);
        if !ptr.is_null() {
            let bytes = std::slice::from_raw_parts(ptr as *const u8, size as usize);
            w.write_string(std::str::from_utf8_unchecked(bytes));
        }
        return Ok(());
    }
    if PyDict_Check(obj) != 0 { return ffi_serialize_dict(py, obj, w, depth, default, allow_nan); }
    if PyList_Check(obj) != 0 { return ffi_serialize_list(py, obj, w, depth, default, allow_nan); }
    if PyTuple_Check(obj) != 0 { return ffi_serialize_tuple(py, obj, w, depth, default, allow_nan); }
    if PyBytes_Check(obj) != 0 {
        let mut size: isize = 0;
        let mut ptr: *mut c_char = std::ptr::null_mut();
        if PyBytes_AsStringAndSize(obj, &mut ptr, &mut size) == 0 && !ptr.is_null() {
            encode_base64_to_buf(w, std::slice::from_raw_parts(ptr as *const u8, size as usize));
        }
        return Ok(());
    }
    if PyByteArray_Check(obj) != 0 {
        let size = PyByteArray_GET_SIZE(obj);
        let ptr = PyByteArray_AS_STRING(obj);
        if !ptr.is_null() {
            encode_base64_to_buf(w, std::slice::from_raw_parts(ptr as *const u8, size as usize));
        }
        return Ok(());
    }
    if PySet_Check(obj) != 0 || PyFrozenSet_Check(obj) != 0 {
        return ffi_serialize_set(py, obj, w, depth, default, allow_nan);
    }
    ffi_serialize_fallback(py, obj, w, depth, default, allow_nan)
}

#[inline(always)]
unsafe fn write_pyrepr(w: &mut JsonWriter, obj: *mut PyObject) {
    let repr = PyObject_Repr(obj);
    if !repr.is_null() {
        let s = PyUnicode_AsUTF8(repr);
        if !s.is_null() { w.buf_mut().push_str(&CStr::from_ptr(s).to_string_lossy()); }
        Py_DECREF(repr);
    }
}

// ── Dict (optimized for string keys) ──────────────────────────────

unsafe fn ffi_serialize_dict(
    py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter,
    depth: usize, default: *mut PyObject, allow_nan: bool,
) -> Result<(), PyErr> {
    if w.sort_keys() {
        let mut keys: Vec<(*mut PyObject, String)> = Vec::new();
        let mut pos: isize = 0;
        let mut key: *mut PyObject = std::ptr::null_mut();
        let mut _value: *mut PyObject = std::ptr::null_mut();
        while PyDict_Next(obj, &mut pos, &mut key, &mut _value) != 0 {
            keys.push((key, extract_key_str(key)));
        }
        keys.sort_by(|a, b| a.1.cmp(&b.1));
        w.write_object_open();
        for (i, (k, _)) in keys.iter().enumerate() {
            if i > 0 { w.write_comma(); }
            write_key(w, *k);
            w.write_colon();
            let value = PyObject_GetItem(obj, *k);
            if !value.is_null() {
                ffi_serialize(py, value, w, depth + 1, default, allow_nan)?;
                Py_DECREF(value);
            }
        }
        w.write_object_close();
    } else {
        w.write_object_open();
        let mut first = true;
        let mut pos: isize = 0;
        let mut key: *mut PyObject = std::ptr::null_mut();
        let mut value: *mut PyObject = std::ptr::null_mut();
        while PyDict_Next(obj, &mut pos, &mut key, &mut value) != 0 {
            if !first { w.write_comma(); }
            write_key(w, key);
            w.write_colon();
            ffi_serialize(py, value, w, depth + 1, default, allow_nan)?;
            first = false;
        }
        w.write_object_close();
    }
    Ok(())
}

#[inline(always)]
unsafe fn extract_key_str(key: *mut PyObject) -> String {
    if PyUnicode_Check(key) != 0 {
        let mut size: isize = 0;
        let ptr = PyUnicode_AsUTF8AndSize(key, &mut size);
        if !ptr.is_null() {
            return std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr as *const u8, size as usize)).to_string();
        }
    } else if PyLong_Check(key) != 0 {
        return PyLong_AsLongLong(key).to_string();
    }
    let repr = PyObject_Str(key);
    let s = if !repr.is_null() {
        let ptr = PyUnicode_AsUTF8(repr);
        let result = if !ptr.is_null() {
            let len = CStr::from_ptr(ptr).to_bytes().len();
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr as *const u8, len)).to_string()
        } else { String::new() };
        Py_DECREF(repr);
        result
    } else { String::new() };
    s
}

/// Fast key write for dict keys. Most keys are strings.
#[inline(always)]
unsafe fn write_key(w: &mut JsonWriter, key: *mut PyObject) {
    if PyUnicode_Check(key) != 0 {
        let mut size: isize = 0;
        let ptr = PyUnicode_AsUTF8AndSize(key, &mut size);
        if !ptr.is_null() {
            let bytes = std::slice::from_raw_parts(ptr as *const u8, size as usize);
            w.write_string(std::str::from_utf8_unchecked(bytes));
        }
    } else if PyLong_Check(key) != 0 {
        let val = PyLong_AsLongLong(key);
        w.buf_mut().push('"');
        let mut buf = itoa::Buffer::new();
        w.buf_mut().push_str(buf.format(val));
        w.buf_mut().push('"');
    } else {
        let repr = PyObject_Str(key);
        if !repr.is_null() {
            let mut size: isize = 0;
            let ptr = PyUnicode_AsUTF8AndSize(repr, &mut size);
            if !ptr.is_null() {
                let bytes = std::slice::from_raw_parts(ptr as *const u8, size as usize);
                w.write_string(std::str::from_utf8_unchecked(bytes));
            }
            Py_DECREF(repr);
        }
    }
}

// ── List / Tuple ──────────────────────────────────────────────────

#[inline(always)]
unsafe fn ffi_serialize_list(
    py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter,
    depth: usize, default: *mut PyObject, allow_nan: bool,
) -> Result<(), PyErr> {
    let len = PyList_GET_SIZE(obj);
    w.write_array_open();
    for i in 0..len {
        if i > 0 { w.write_comma(); }
        ffi_serialize(py, PyList_GET_ITEM(obj, i), w, depth + 1, default, allow_nan)?;
    }
    w.write_array_close();
    Ok(())
}

#[inline(always)]
unsafe fn ffi_serialize_tuple(
    py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter,
    depth: usize, default: *mut PyObject, allow_nan: bool,
) -> Result<(), PyErr> {
    let len = PyTuple_GET_SIZE(obj);
    w.write_array_open();
    for i in 0..len {
        if i > 0 { w.write_comma(); }
        ffi_serialize(py, PyTuple_GET_ITEM(obj, i), w, depth + 1, default, allow_nan)?;
    }
    w.write_array_close();
    Ok(())
}

// ── Set / FrozenSet ───────────────────────────────────────────────

unsafe fn ffi_serialize_set(
    py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter,
    depth: usize, default: *mut PyObject, allow_nan: bool,
) -> Result<(), PyErr> {
    let mut items: Vec<(String, *mut PyObject)> = Vec::new();
    let iter = PyObject_GetIter(obj);
    if iter.is_null() { return Ok(()); }
    loop {
        let item = PyIter_Next(iter);
        if item.is_null() { break; }
        let s = {
            let repr = PyObject_Str(item);
            if !repr.is_null() {
                let ptr = PyUnicode_AsUTF8(repr);
                let result = if !ptr.is_null() {
                    let len = CStr::from_ptr(ptr).to_bytes().len();
                    std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr as *const u8, len)).to_string()
                } else { String::new() };
                Py_DECREF(repr);
                result
            } else { String::new() }
        };
        items.push((s, item));
    }
    Py_DECREF(iter);
    items.sort_by(|a, b| a.0.cmp(&b.0));
    w.write_array_open();
    for (i, (_, item)) in items.iter().enumerate() {
        if i > 0 { w.write_comma(); }
        ffi_serialize(py, *item, w, depth + 1, default, allow_nan)?;
    }
    w.write_array_close();
    for (_, item) in items { Py_DECREF(item); }
    Ok(())
}

// ── Fallback (reordered by frequency) ─────────────────────────────

unsafe fn ffi_serialize_fallback(
    py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter,
    depth: usize, default: *mut PyObject, allow_nan: bool,
) -> Result<(), PyErr> {
    let obj_bound = Bound::from_borrowed_ptr(py, obj);

    // Check cached types (ordered by typical usage frequency)
    if let Some(ct) = get_cached_types(py) {
        // datetime (most common complex type in Django)
        if !ct.datetime_type.is_null() && PyObject_IsInstance(obj, ct.datetime_type) == 1 {
            write_datetime(obj, w)?;
            return Ok(());
        }
        // UUID (common in Django)
        if !ct.uuid_type.is_null() && PyObject_IsInstance(obj, ct.uuid_type) == 1 {
            write_pystr(obj, w);
            return Ok(());
        }
        // Decimal (common in Django)
        if !ct.decimal_type.is_null() && PyObject_IsInstance(obj, ct.decimal_type) == 1 {
            write_pystr(obj, w);
            return Ok(());
        }
        // date
        if !ct.date_type.is_null() && PyObject_IsInstance(obj, ct.date_type) == 1 {
            let iso = obj_bound.call_method0("isoformat")?;
            let s: String = iso.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        // time
        if !ct.time_type.is_null() && PyObject_IsInstance(obj, ct.time_type) == 1 {
            let iso = obj_bound.call_method0("isoformat")?;
            let s: String = iso.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        // timedelta
        if !ct.timedelta_type.is_null() && PyObject_IsInstance(obj, ct.timedelta_type) == 1 {
            write_timedelta(obj, w)?;
            return Ok(());
        }
        // Promise
        if !ct.promise.is_null() && PyObject_IsInstance(obj, ct.promise) == 1 {
            write_pystr(obj, w);
            return Ok(());
        }
        // Model
        if !ct.model.is_null() && PyObject_IsInstance(obj, ct.model) == 1 {
            return ffi_serialize_model(py, &obj_bound, w, depth, default, allow_nan);
        }
        // QuerySet
        if !ct.queryset.is_null() && PyObject_IsInstance(obj, ct.queryset) == 1 {
            return ffi_serialize_queryset(py, &obj_bound, w, depth, default, allow_nan);
        }
        // Enum
        if !ct.enum_type.is_null() && PyObject_IsInstance(obj, ct.enum_type) == 1 {
            let val = obj_bound.getattr("value")?;
            ffi_serialize(py, val.as_ptr(), w, depth, default, allow_nan)?;
            return Ok(());
        }
    }

    // dataclass
    if obj_bound.hasattr("__dataclass_fields__").unwrap_or(false) {
        let dict = py.import("dataclasses")?.call_method1("asdict", (&obj_bound,))?;
        ffi_serialize(py, dict.as_ptr(), w, depth, default, allow_nan)?;
        return Ok(());
    }

    // Recursive default
    if !default.is_null() {
        let bound_default = Bound::from_borrowed_ptr(py, default);
        let result = bound_default.call1((&obj_bound,))?;
        ffi_serialize(py, result.as_ptr(), w, depth, default, allow_nan)?;
        return Ok(());
    }

    let type_name = obj_bound.get_type().name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    Err(pyo3::exceptions::PyTypeError::new_err(format!(
        "Object of type {} is not JSON serializable", type_name
    )))
}

// ── DateTime FFI direct ───────────────────────────────────────────

#[inline(always)]
unsafe fn write_datetime(obj: *mut PyObject, w: &mut JsonWriter) -> Result<(), PyErr> {
    let y = PyDateTime_GET_YEAR(obj);
    let mo = PyDateTime_GET_MONTH(obj);
    let d = PyDateTime_GET_DAY(obj);
    let h = PyDateTime_DATE_GET_HOUR(obj);
    let mi = PyDateTime_DATE_GET_MINUTE(obj);
    let s = PyDateTime_DATE_GET_SECOND(obj);
    let us = PyDateTime_DATE_GET_MICROSECOND(obj);
    let tzinfo = PyDateTime_DATE_GET_TZINFO(obj);
    let has_tz = !tzinfo.is_null() && Py_IsNone(tzinfo) == 0;

    let is_utc = if has_tz {
        let dt_mod = PyImport_ImportModule(b"datetime\0".as_ptr() as *const c_char);
        if !dt_mod.is_null() {
            let tz_cls = PyObject_GetAttrString(dt_mod, b"timezone\0".as_ptr() as *const c_char);
            let result = if !tz_cls.is_null() {
                let utc_val = PyObject_GetAttrString(tz_cls, b"utc\0".as_ptr() as *const c_char);
                let r = if !utc_val.is_null() {
                    let eq = PyObject_RichCompareBool(tzinfo, utc_val, Py_EQ);
                    Py_DECREF(utc_val);
                    eq == 1
                } else { false };
                Py_DECREF(tz_cls);
                r
            } else { false };
            Py_DECREF(dt_mod);
            result
        } else { false }
    } else { false };

    let mut buf = String::with_capacity(32);
    if us > 0 {
        write!(buf, "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}", y, mo, d, h, mi, s, us).unwrap();
    } else {
        write!(buf, "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", y, mo, d, h, mi, s).unwrap();
    }
    if is_utc { buf.push('Z'); }
    w.write_string(&buf);
    Ok(())
}

// ── Timedelta FFI direct ──────────────────────────────────────────

#[inline(always)]
unsafe fn write_timedelta(obj: *mut PyObject, w: &mut JsonWriter) -> Result<(), PyErr> {
    let days = PyDateTime_DELTA_GET_DAYS(obj);
    let total_secs = PyDateTime_DELTA_GET_SECONDS(obj);
    let micros = PyDateTime_DELTA_GET_MICROSECONDS(obj);
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    let mut buf = String::with_capacity(32);
    buf.push('P');
    if days != 0 { write!(buf, "{}D", days).unwrap(); }
    if hours != 0 || minutes != 0 || secs != 0 || micros != 0 {
        buf.push('T');
        if hours != 0 { write!(buf, "{}H", hours).unwrap(); }
        if minutes != 0 { write!(buf, "{}M", minutes).unwrap(); }
        if micros > 0 { write!(buf, "{}.{:06}S", secs, micros).unwrap(); }
        else if secs != 0 { write!(buf, "{}S", secs).unwrap(); }
    }
    if buf == "P" || buf == "PT" { buf.push_str("T0S"); }
    w.write_string(&buf);
    Ok(())
}

// ── UUID/Decimal str() ────────────────────────────────────────────

#[inline(always)]
unsafe fn write_pystr(obj: *mut PyObject, w: &mut JsonWriter) {
    let s = PyObject_Str(obj);
    if !s.is_null() {
        let mut size: isize = 0;
        let ptr = PyUnicode_AsUTF8AndSize(s, &mut size);
        if !ptr.is_null() {
            let bytes = std::slice::from_raw_parts(ptr as *const u8, size as usize);
            w.write_string(std::str::from_utf8_unchecked(bytes));
        }
        Py_DECREF(s);
    }
}

// ── Model / QuerySet ──────────────────────────────────────────────

unsafe fn ffi_serialize_model(
    py: Python<'_>, obj: &Bound<'_, PyAny>, w: &mut JsonWriter,
    depth: usize, default: *mut PyObject, allow_nan: bool,
) -> Result<(), PyErr> {
    let meta = obj.getattr("_meta")?;
    let fields = meta.getattr("fields")?;
    let iter = fields.call_method0("__iter__")?;
    let py_iter = pyo3::types::PyIterator::from_object(&iter)?;
    w.write_object_open();
    let mut first = true;
    for fr in py_iter {
        let field = fr?;
        let field_name: String = field.getattr("name")?.extract()?;
        if !first { w.write_comma(); }
        w.write_string(&field_name);
        w.write_colon();
        let value = obj.getattr(field_name.as_str())?;
        ffi_serialize(py, value.as_ptr(), w, depth + 1, default, allow_nan)?;
        first = false;
    }
    w.write_object_close();
    Ok(())
}

unsafe fn ffi_serialize_queryset(
    py: Python<'_>, qs: &Bound<'_, PyAny>, w: &mut JsonWriter,
    depth: usize, default: *mut PyObject, allow_nan: bool,
) -> Result<(), PyErr> {
    let iterator = pyo3::types::PyIterator::from_object(qs)?;
    w.write_array_open();
    let mut first = true;
    for item_result in iterator {
        let item = item_result?;
        if !first { w.write_comma(); }
        ffi_serialize(py, item.as_ptr(), w, depth + 1, default, allow_nan)?;
        first = false;
    }
    w.write_array_close();
    Ok(())
}

// ── Base64 ────────────────────────────────────────────────────────

const B64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[inline]
fn encode_base64_to_buf(w: &mut JsonWriter, data: &[u8]) {
    w.buf_mut().push('"');
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        w.buf_mut().push(B64_CHARS[((triple >> 18) & 0x3F) as usize] as char);
        w.buf_mut().push(B64_CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 { w.buf_mut().push(B64_CHARS[((triple >> 6) & 0x3F) as usize] as char); }
        else { w.buf_mut().push('='); }
        if chunk.len() > 2 { w.buf_mut().push(B64_CHARS[(triple & 0x3F) as usize] as char); }
        else { w.buf_mut().push('='); }
    }
    w.buf_mut().push('"');
}
