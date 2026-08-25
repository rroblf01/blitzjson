use std::ffi::CStr;
use pyo3::prelude::*;
use pyo3::ffi::*;
use std::os::raw::{c_char, c_int};
use crate::writer::JsonWriter;

const MAX_DEPTH: usize = 128;

struct DjangoTypes {
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
unsafe impl Send for DjangoTypes {}
unsafe impl Sync for DjangoTypes {}
static mut DJANGO_TYPES: Option<DjangoTypes> = None;

unsafe fn get_django_types(py: Python<'_>) -> Option<&'static DjangoTypes> {
    if DJANGO_TYPES.is_some() { return DJANGO_TYPES.as_ref(); }
    let promise = py.import("django.utils.functional").ok()
        .and_then(|m| m.getattr("Promise").ok()).map(|o| o.as_ptr());
    let model = py.import("django.db.models").ok()
        .and_then(|m| m.getattr("Model").ok()).map(|o| o.as_ptr());
    let queryset = py.import("django.db.models.query").ok()
        .and_then(|m| m.getattr("QuerySet").ok()).map(|o| o.as_ptr());
    let enum_type = py.import("enum").ok()
        .and_then(|m| m.getattr("Enum").ok()).map(|o| o.as_ptr());
    let datetime_type = py.import("datetime").ok()
        .and_then(|m| m.getattr("datetime").ok()).map(|o| o.as_ptr());
    let date_type = py.import("datetime").ok()
        .and_then(|m| m.getattr("date").ok()).map(|o| o.as_ptr());
    let time_type = py.import("datetime").ok()
        .and_then(|m| m.getattr("time").ok()).map(|o| o.as_ptr());
    let timedelta_type = py.import("datetime").ok()
        .and_then(|m| m.getattr("timedelta").ok()).map(|o| o.as_ptr());
    let uuid_type = py.import("uuid").ok()
        .and_then(|m| m.getattr("UUID").ok()).map(|o| o.as_ptr());
    let decimal_type = py.import("decimal").ok()
        .and_then(|m| m.getattr("Decimal").ok()).map(|o| o.as_ptr());
    DJANGO_TYPES = Some(DjangoTypes {
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
    DJANGO_TYPES.as_ref()
}

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
        else {
            let repr = PyObject_Repr(obj);
            if !repr.is_null() {
                let s = PyUnicode_AsUTF8(repr);
                if !s.is_null() { w.buf_mut().push_str(&CStr::from_ptr(s).to_string_lossy()); }
                Py_DECREF(repr);
            }
        }
        return Ok(());
    }
    if PyFloat_Check(obj) != 0 {
        let val = PyFloat_AS_DOUBLE(obj);
        if val.is_nan() {
            if allow_nan { w.write_raw("NaN"); }
            else { return Err(pyo3::exceptions::PyValueError::new_err("Out of range float values are not JSON compliant")); }
        } else if val.is_infinite() {
            if allow_nan {
                if val > 0.0 { w.write_raw("Infinity"); }
                else { w.write_raw("-Infinity"); }
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

unsafe fn ffi_serialize_dict(
    py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter,
    depth: usize, default: *mut PyObject, allow_nan: bool,
) -> Result<(), PyErr> {
    if w.sort_keys() {
        let mut keys: Vec<*mut PyObject> = Vec::new();
        let mut pos: isize = 0;
        let mut key: *mut PyObject = std::ptr::null_mut();
        let mut _value: *mut PyObject = std::ptr::null_mut();
        while PyDict_Next(obj, &mut pos, &mut key, &mut _value) != 0 { keys.push(key); }
        keys.sort_by(|a, b| {
            let a_s = PyObject_Str(*a);
            let b_s = PyObject_Str(*b);
            let result = if !a_s.is_null() && !b_s.is_null() {
                let a_ptr = PyUnicode_AsUTF8(a_s);
                let b_ptr = PyUnicode_AsUTF8(b_s);
                if !a_ptr.is_null() && !b_ptr.is_null() { CStr::from_ptr(a_ptr).cmp(CStr::from_ptr(b_ptr)) }
                else { std::cmp::Ordering::Equal }
            } else { std::cmp::Ordering::Equal };
            if !a_s.is_null() { Py_DECREF(a_s); }
            if !b_s.is_null() { Py_DECREF(b_s); }
            result
        });
        w.write_object_open();
        for (i, k) in keys.iter().enumerate() {
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

#[inline]
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

unsafe fn ffi_serialize_set(
    py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter,
    depth: usize, default: *mut PyObject, allow_nan: bool,
) -> Result<(), PyErr> {
    let mut items: Vec<(*mut PyObject, *mut PyObject)> = Vec::new();
    let iter = PyObject_GetIter(obj);
    if iter.is_null() { return Ok(()); }
    loop {
        let item = PyIter_Next(iter);
        if item.is_null() { break; }
        let s = PyObject_Str(item);
        items.push((s, item));
    }
    Py_DECREF(iter);
    items.sort_by(|a, b| {
        if a.0.is_null() || b.0.is_null() { return std::cmp::Ordering::Equal; }
        let a_ptr = PyUnicode_AsUTF8(a.0);
        let b_ptr = PyUnicode_AsUTF8(b.0);
        if a_ptr.is_null() || b_ptr.is_null() { return std::cmp::Ordering::Equal; }
        CStr::from_ptr(a_ptr).cmp(CStr::from_ptr(b_ptr))
    });
    w.write_array_open();
    for (i, (_, item)) in items.iter().enumerate() {
        if i > 0 { w.write_comma(); }
        ffi_serialize(py, *item, w, depth + 1, default, allow_nan)?;
    }
    w.write_array_close();
    for (s, item) in items {
        if !s.is_null() { Py_DECREF(s); }
        Py_DECREF(item);
    }
    Ok(())
}

unsafe fn ffi_serialize_fallback(
    py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter,
    depth: usize, default: *mut PyObject, allow_nan: bool,
) -> Result<(), PyErr> {
    let obj_bound = Bound::from_borrowed_ptr(py, obj);

    // Fast path: use cached type pointers for comparison (no allocation)
    if let Some(dt) = get_django_types(py) {
        // datetime (check before date since datetime is a subclass of date)
        if !dt.datetime_type.is_null() && PyObject_IsInstance(obj, dt.datetime_type) == 1 {
            let iso = obj_bound.call_method0("isoformat")?;
            let s: String = iso.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        // date
        if !dt.date_type.is_null() && PyObject_IsInstance(obj, dt.date_type) == 1 {
            let iso = obj_bound.call_method0("isoformat")?;
            let s: String = iso.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        // time
        if !dt.time_type.is_null() && PyObject_IsInstance(obj, dt.time_type) == 1 {
            let iso = obj_bound.call_method0("isoformat")?;
            let s: String = iso.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        // timedelta
        if !dt.timedelta_type.is_null() && PyObject_IsInstance(obj, dt.timedelta_type) == 1 {
            let days: i64 = obj_bound.getattr("days")?.extract()?;
            let seconds: i64 = obj_bound.getattr("seconds")?.extract()?;
            let microseconds: i64 = obj_bound.getattr("microseconds")?.extract()?;
            let hours = seconds / 3600;
            let minutes = (seconds % 3600) / 60;
            let secs = seconds % 60;
            let micros = microseconds as u32;
            let mut result = String::with_capacity(32);
            result.push('P');
            if days != 0 { use std::fmt::Write; result.write_fmt(format_args!("{}D", days)).unwrap(); }
            if hours != 0 || minutes != 0 || secs != 0 || micros != 0 {
                result.push('T');
                if hours != 0 { use std::fmt::Write; result.write_fmt(format_args!("{}H", hours)).unwrap(); }
                if minutes != 0 { use std::fmt::Write; result.write_fmt(format_args!("{}M", minutes)).unwrap(); }
                if micros > 0 { use std::fmt::Write; result.write_fmt(format_args!("{}.{:06}S", secs, micros)).unwrap(); }
                else if secs != 0 { use std::fmt::Write; result.write_fmt(format_args!("{}S", secs)).unwrap(); }
            }
            if result == "P" { result.push_str("T0S"); }
            w.write_string(&result);
            return Ok(());
        }
        // UUID
        if !dt.uuid_type.is_null() && PyObject_IsInstance(obj, dt.uuid_type) == 1 {
            let s: String = obj_bound.str()?.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        // Decimal
        if !dt.decimal_type.is_null() && PyObject_IsInstance(obj, dt.decimal_type) == 1 {
            let s: String = obj_bound.str()?.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        // Promise
        if !dt.promise.is_null() && PyObject_IsInstance(obj, dt.promise) == 1 {
            let s: String = obj_bound.str()?.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        // Model
        if !dt.model.is_null() && PyObject_IsInstance(obj, dt.model) == 1 {
            return ffi_serialize_model(py, &obj_bound, w, depth, default, allow_nan);
        }
        // QuerySet
        if !dt.queryset.is_null() && PyObject_IsInstance(obj, dt.queryset) == 1 {
            return ffi_serialize_queryset(py, &obj_bound, w, depth, default, allow_nan);
        }
        // Enum
        if !dt.enum_type.is_null() && PyObject_IsInstance(obj, dt.enum_type) == 1 {
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

    // Last resort: use type name for error message only
    let type_name = obj_bound.get_type().name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    Err(pyo3::exceptions::PyTypeError::new_err(format!(
        "Object of type {} is not JSON serializable", type_name
    )))
}

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
