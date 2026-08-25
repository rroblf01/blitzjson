use std::ffi::CStr;
use pyo3::prelude::*;
use pyo3::ffi::*;
use std::os::raw::{c_char, c_int};
use crate::writer::JsonWriter;

const MAX_DEPTH: usize = 128;

pub unsafe fn ffi_serialize(
    py: Python<'_>,
    obj: *mut PyObject,
    w: &mut JsonWriter,
    depth: usize,
) -> Result<(), PyErr> {
    if depth >= MAX_DEPTH {
        return Err(pyo3::exceptions::PyValueError::new_err("Maximum recursion depth exceeded"));
    }
    if obj.is_null() || Py_IsNone(obj) != 0 {
        w.write_none();
        return Ok(());
    }
    if PyBool_Check(obj) != 0 {
        w.write_bool(obj == Py_True());
        return Ok(());
    }
    if PyLong_Check(obj) != 0 {
        let mut overflow: c_int = 0;
        let val = PyLong_AsLongLongAndOverflow(obj, &mut overflow);
        if overflow == 0 {
            w.write_i64(val as i64);
        } else {
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
        if val.is_nan() || val.is_infinite() {
            return Err(pyo3::exceptions::PyValueError::new_err("Out of range float values are not JSON compliant"));
        }
        w.write_f64(val);
        return Ok(());
    }
    if PyUnicode_Check(obj) != 0 {
        let mut size: isize = 0;
        let ptr = PyUnicode_AsUTF8AndSize(obj, &mut size);
        if !ptr.is_null() {
            let bytes = std::slice::from_raw_parts(ptr as *const u8, size as usize);
            w.write_string(std::str::from_utf8_unchecked(bytes));
            return Ok(());
        }
        return Ok(());
    }
    if PyDict_Check(obj) != 0 { return ffi_serialize_dict(py, obj, w, depth); }
    if PyList_Check(obj) != 0 { return ffi_serialize_list(py, obj, w, depth); }
    if PyTuple_Check(obj) != 0 { return ffi_serialize_tuple(py, obj, w, depth); }
    if PyBytes_Check(obj) != 0 {
        let mut size: isize = 0;
        let mut ptr: *mut c_char = std::ptr::null_mut();
        if PyBytes_AsStringAndSize(obj, &mut ptr, &mut size) == 0 && !ptr.is_null() {
            let bytes = std::slice::from_raw_parts(ptr as *const u8, size as usize);
            w.write_string(&encode_base64(bytes));
        }
        return Ok(());
    }
    if PyByteArray_Check(obj) != 0 {
        let size = PyByteArray_GET_SIZE(obj);
        let ptr = PyByteArray_AS_STRING(obj);
        if !ptr.is_null() {
            let bytes = std::slice::from_raw_parts(ptr as *const u8, size as usize);
            w.write_string(&encode_base64(bytes));
        }
        return Ok(());
    }
    if PySet_Check(obj) != 0 || PyFrozenSet_Check(obj) != 0 { return ffi_serialize_set(py, obj, w, depth); }
    ffi_serialize_fallback(py, obj, w, depth)
}

unsafe fn ffi_serialize_dict(py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter, depth: usize) -> Result<(), PyErr> {
    w.write_object_open();
    let mut first = true;
    let mut pos: isize = 0;
    let mut key: *mut PyObject = std::ptr::null_mut();
    let mut value: *mut PyObject = std::ptr::null_mut();
    while PyDict_Next(obj, &mut pos, &mut key, &mut value) != 0 {
        if !first { w.write_comma(); }
        if PyUnicode_Check(key) != 0 {
            let mut size: isize = 0;
            let ptr = PyUnicode_AsUTF8AndSize(key, &mut size);
            if !ptr.is_null() {
                let bytes = std::slice::from_raw_parts(ptr as *const u8, size as usize);
                w.write_string(std::str::from_utf8_unchecked(bytes));
            }
        } else if PyLong_Check(key) != 0 {
            let val = PyLong_AsLongLong(key);
            let key_str = val.to_string();
            w.write_string(&key_str);
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
        w.write_colon();
        ffi_serialize(py, value, w, depth + 1)?;
        first = false;
    }
    w.write_object_close();
    Ok(())
}

unsafe fn ffi_serialize_list(py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter, depth: usize) -> Result<(), PyErr> {
    let len = PyList_GET_SIZE(obj);
    w.write_array_open();
    for i in 0..len {
        if i > 0 { w.write_comma(); }
        ffi_serialize(py, PyList_GET_ITEM(obj, i), w, depth + 1)?;
    }
    w.write_array_close();
    Ok(())
}

unsafe fn ffi_serialize_tuple(py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter, depth: usize) -> Result<(), PyErr> {
    let len = PyTuple_GET_SIZE(obj);
    w.write_array_open();
    for i in 0..len {
        if i > 0 { w.write_comma(); }
        ffi_serialize(py, PyTuple_GET_ITEM(obj, i), w, depth + 1)?;
    }
    w.write_array_close();
    Ok(())
}

unsafe fn ffi_serialize_set(py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter, depth: usize) -> Result<(), PyErr> {
    let mut items: Vec<*mut PyObject> = Vec::new();
    let iter = PyObject_GetIter(obj);
    if iter.is_null() { return Ok(()); }
    loop {
        let item = PyIter_Next(iter);
        if item.is_null() { break; }
        items.push(item);
    }
    Py_DECREF(iter);
    items.sort_by(|a, b| {
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
    w.write_array_open();
    for (i, item) in items.iter().enumerate() {
        if i > 0 { w.write_comma(); }
        ffi_serialize(py, *item, w, depth + 1)?;
    }
    w.write_array_close();
    for item in items { Py_DECREF(item); }
    Ok(())
}

unsafe fn ffi_serialize_fallback(py: Python<'_>, obj: *mut PyObject, w: &mut JsonWriter, depth: usize) -> Result<(), PyErr> {
    let obj_bound = Bound::from_borrowed_ptr(py, obj);

    // Use pyo3 safe API for complex types (datetime, UUID, Decimal, Django types)
    // These are slower but correct - the fast FFI path handles the hot types
    let type_name = obj_bound.get_type().name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

    match type_name.as_str() {
        "datetime" => {
            let iso = obj_bound.call_method0("isoformat")?;
            let s: String = iso.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        "date" => {
            let iso = obj_bound.call_method0("isoformat")?;
            let s: String = iso.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        "time" => {
            let iso = obj_bound.call_method0("isoformat")?;
            let s: String = iso.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        "timedelta" => {
            let days: i64 = obj_bound.getattr("days")?.extract()?;
            let seconds: i64 = obj_bound.getattr("seconds")?.extract()?;
            let microseconds: i64 = obj_bound.getattr("microseconds")?.extract()?;
            let hours = seconds / 3600;
            let minutes = (seconds % 3600) / 60;
            let secs = seconds % 60;
            let micros = microseconds as u32;
            let mut result = String::from("P");
            if days != 0 { result.push_str(&format!("{}D", days)); }
            if hours != 0 || minutes != 0 || secs != 0 || micros != 0 {
                result.push_str("T");
                if hours != 0 { result.push_str(&format!("{}H", hours)); }
                if minutes != 0 { result.push_str(&format!("{}M", minutes)); }
                if micros > 0 { result.push_str(&format!("{}.{:06}S", secs, micros)); }
                else { result.push_str(&format!("{}S", secs)); }
            }
            if result == "P" { result.push_str("T0S"); }
            w.write_string(&result);
            return Ok(());
        }
        "UUID" => {
            let s: String = obj_bound.str()?.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        "Decimal" => {
            let s: String = obj_bound.str()?.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        _ => {}
    }

    // Django types
    if let Ok(m) = py.import("django.utils.functional") {
        if let Ok(pt) = m.getattr("Promise") {
            if obj_bound.is_instance(&pt).unwrap_or(false) {
                let s: String = obj_bound.str()?.extract()?;
                w.write_string(&s);
                return Ok(());
            }
        }
    }
    if let Ok(m) = py.import("django.db.models") {
        if let Ok(mt) = m.getattr("Model") {
            if obj_bound.is_instance(&mt).unwrap_or(false) {
                return ffi_serialize_model(py, &obj_bound, w, depth);
            }
        }
    }
    if let Ok(m) = py.import("django.db.models.query") {
        if let Ok(qt) = m.getattr("QuerySet") {
            if obj_bound.is_instance(&qt).unwrap_or(false) {
                return ffi_serialize_queryset(py, &obj_bound, w, depth);
            }
        }
    }
    if let Ok(m) = py.import("enum") {
        if let Ok(et) = m.getattr("Enum") {
            if obj_bound.is_instance(&et).unwrap_or(false) {
                let val = obj_bound.getattr("value")?;
                ffi_serialize(py, val.as_ptr(), w, depth)?;
                return Ok(());
            }
        }
    }
    if obj_bound.hasattr("__dataclass_fields__").unwrap_or(false) {
        let dict = py.import("dataclasses")?.call_method1("asdict", (&obj_bound,))?;
        ffi_serialize(py, dict.as_ptr(), w, depth)?;
        return Ok(());
    }
    let s: String = obj_bound.str()?.extract()?;
    w.write_string(&s);
    Ok(())
}

unsafe fn ffi_serialize_model(py: Python<'_>, obj: &Bound<'_, PyAny>, w: &mut JsonWriter, depth: usize) -> Result<(), PyErr> {
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
        ffi_serialize(py, value.as_ptr(), w, depth + 1)?;
        first = false;
    }
    w.write_object_close();
    Ok(())
}

unsafe fn ffi_serialize_queryset(py: Python<'_>, qs: &Bound<'_, PyAny>, w: &mut JsonWriter, depth: usize) -> Result<(), PyErr> {
    let iterator = pyo3::types::PyIterator::from_object(qs)?;
    w.write_array_open();
    let mut first = true;
    for item_result in iterator {
        let item = item_result?;
        if !first { w.write_comma(); }
        ffi_serialize(py, item.as_ptr(), w, depth + 1)?;
        first = false;
    }
    w.write_array_close();
    Ok(())
}

fn encode_base64(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 { result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char); }
        else { result.push('='); }
        if chunk.len() > 2 { result.push(CHARS[(triple & 0x3F) as usize] as char); }
        else { result.push('='); }
    }
    result
}
