use crate::writer::JsonWriter;
use pyo3::ffi::PyObject;
use pyo3::prelude::*;
use pyo3::types::PyDict;

const MAX_DEPTH: usize = 128;

pub unsafe fn ffi_serialize(
    py: Python<'_>,
    obj: *mut PyObject,
    w: &mut JsonWriter,
    depth: usize,
    default: *mut PyObject,
    allow_nan: bool,
    sort_keys: bool,
) -> Result<(), PyErr> {
    if depth >= MAX_DEPTH {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Maximum recursion depth exceeded",
        ));
    }
    let obj_bound = Bound::from_borrowed_ptr(py, obj);
    serialize_value(py, &obj_bound, w, depth, default, allow_nan, sort_keys)
}

unsafe fn serialize_value(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    w: &mut JsonWriter,
    depth: usize,
    default: *mut PyObject,
    allow_nan: bool,
    sort_keys: bool,
) -> Result<(), PyErr> {
    if obj.is_none() {
        w.write_none();
        return Ok(());
    }
    if let Ok(b) = obj.extract::<bool>() {
        w.write_bool(b);
        return Ok(());
    }
    if let Ok(s) = obj.extract::<String>() {
        w.write_string(&s);
        return Ok(());
    }
    let type_name = obj
        .get_type()
        .name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if type_name == "Decimal" || type_name == "decimal.Decimal" || type_name == "UUID" {
        let s: String = obj.str()?.extract()?;
        w.write_string(&s);
        return Ok(());
    }
    if let Ok(i) = obj.extract::<i64>() {
        w.write_i64(i);
        return Ok(());
    }
    if let Ok(f) = obj.extract::<f64>() {
        if f.is_nan() || f.is_infinite() {
            if allow_nan {
                if f.is_nan() {
                    w.write_raw("NaN");
                } else if f > 0.0 {
                    w.write_raw("Infinity");
                } else {
                    w.write_raw("-Infinity");
                }
            } else {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "Out of range float values are not JSON compliant",
                ));
            }
        } else {
            w.write_f64(f);
        }
        return Ok(());
    }
    if let Ok(dict) = obj.cast::<PyDict>() {
        w.write_object_open();
        if sort_keys {
            let mut items: Vec<(String, Bound<'_, PyAny>)> = Vec::new();
            for (k, v) in dict.iter() {
                let key: String = k.extract()?;
                items.push((key, v));
            }
            items.sort_by(|a, b| a.0.cmp(&b.0));
            for (i, (key, v)) in items.iter().enumerate() {
                if i > 0 {
                    w.write_comma();
                }
                w.write_string(key);
                w.write_colon();
                serialize_value(py, v, w, depth + 1, default, allow_nan, sort_keys)?;
            }
        } else {
            let mut first = true;
            for (k, v) in dict.iter() {
                if !first {
                    w.write_comma();
                }
                let key: String = k.extract()?;
                w.write_string(&key);
                w.write_colon();
                serialize_value(py, &v, w, depth + 1, default, allow_nan, sort_keys)?;
                first = false;
            }
        }
        w.write_object_close();
        return Ok(());
    }
    if let Ok(list) = obj.cast::<pyo3::types::PyList>() {
        w.write_array_open();
        for (i, item) in list.iter().enumerate() {
            if i > 0 {
                w.write_comma();
            }
            serialize_value(py, &item, w, depth + 1, default, allow_nan, sort_keys)?;
        }
        w.write_array_close();
        return Ok(());
    }
    if let Ok(tuple) = obj.cast::<pyo3::types::PyTuple>() {
        w.write_array_open();
        for (i, item) in tuple.iter().enumerate() {
            if i > 0 {
                w.write_comma();
            }
            serialize_value(py, &item, w, depth + 1, default, allow_nan, sort_keys)?;
        }
        w.write_array_close();
        return Ok(());
    }
    if let Ok(set) = obj.cast::<pyo3::types::PySet>() {
        let mut items: Vec<Bound<'_, PyAny>> = set.iter().collect();
        items.sort_by(|a, b| {
            a.str()
                .unwrap()
                .to_string()
                .cmp(&b.str().unwrap().to_string())
        });
        w.write_array_open();
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                w.write_comma();
            }
            serialize_value(py, item, w, depth + 1, default, allow_nan, sort_keys)?;
        }
        w.write_array_close();
        return Ok(());
    }
    if let Ok(fset) = obj.cast::<pyo3::types::PyFrozenSet>() {
        let mut items: Vec<Bound<'_, PyAny>> = fset.iter().collect();
        items.sort_by(|a, b| {
            a.str()
                .unwrap()
                .to_string()
                .cmp(&b.str().unwrap().to_string())
        });
        w.write_array_open();
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                w.write_comma();
            }
            serialize_value(py, item, w, depth + 1, default, allow_nan, sort_keys)?;
        }
        w.write_array_close();
        return Ok(());
    }
    if let Ok(bytes) = obj.cast::<pyo3::types::PyBytes>() {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes.as_bytes());
        w.write_string(&encoded);
        return Ok(());
    }
    if let Ok(ba) = obj.cast::<pyo3::types::PyByteArray>() {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(ba.as_bytes());
        w.write_string(&encoded);
        return Ok(());
    }
    serialize_fallback(py, obj, w, depth, default, allow_nan, sort_keys)
}

unsafe fn serialize_fallback(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    w: &mut JsonWriter,
    depth: usize,
    default: *mut PyObject,
    allow_nan: bool,
    sort_keys: bool,
) -> Result<(), PyErr> {
    let type_name = obj
        .get_type()
        .name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    match type_name.as_str() {
        "datetime" | "date" | "time" => {
            let iso = obj.call_method0("isoformat")?;
            let s: String = iso.extract()?;
            w.write_string(&s);
            return Ok(());
        }
        "timedelta" => {
            let days: i64 = obj.getattr("days")?.extract()?;
            let seconds: i64 = obj.getattr("seconds")?.extract()?;
            let microseconds: i64 = obj.getattr("microseconds")?.extract()?;
            let hours = seconds / 3600;
            let minutes = (seconds % 3600) / 60;
            let secs = seconds % 60;
            let micros = microseconds as u32;
            let mut result = String::with_capacity(32);
            result.push('P');
            if days != 0 {
                use std::fmt::Write;
                result.write_fmt(format_args!("{}D", days)).unwrap();
            }
            if hours != 0 || minutes != 0 || secs != 0 || micros != 0 {
                result.push('T');
                if hours != 0 {
                    use std::fmt::Write;
                    result.write_fmt(format_args!("{}H", hours)).unwrap();
                }
                if minutes != 0 {
                    use std::fmt::Write;
                    result.write_fmt(format_args!("{}M", minutes)).unwrap();
                }
                if micros > 0 {
                    use std::fmt::Write;
                    result
                        .write_fmt(format_args!("{}.{:06}S", secs, micros))
                        .unwrap();
                } else if secs != 0 {
                    use std::fmt::Write;
                    result.write_fmt(format_args!("{}S", secs)).unwrap();
                }
            }
            if result == "P" || result == "PT" {
                result.push_str("T0S");
            }
            w.write_string(&result);
            return Ok(());
        }
        _ => {}
    }
    if let Ok(dt) = py.import("django.utils.functional") {
        if let Ok(pt) = dt.getattr("Promise") {
            if obj.is_instance(&pt).unwrap_or(false) {
                let s: String = obj.str()?.extract()?;
                w.write_string(&s);
                return Ok(());
            }
        }
    }
    if let Ok(m) = py.import("django.db.models") {
        if let Ok(mt) = m.getattr("Model") {
            if obj.is_instance(&mt).unwrap_or(false) {
                return serialize_model(py, obj, w, depth, default, allow_nan, sort_keys);
            }
        }
    }
    if let Ok(m) = py.import("django.db.models.query") {
        if let Ok(qt) = m.getattr("QuerySet") {
            if obj.is_instance(&qt).unwrap_or(false) {
                return serialize_queryset(py, obj, w, depth, default, allow_nan, sort_keys);
            }
        }
    }
    if let Ok(m) = py.import("enum") {
        if let Ok(et) = m.getattr("Enum") {
            if obj.is_instance(&et).unwrap_or(false) {
                let val = obj.getattr("value")?;
                serialize_value(py, &val, w, depth, default, allow_nan, sort_keys)?;
                return Ok(());
            }
        }
    }
    if obj.hasattr("__dataclass_fields__").unwrap_or(false) {
        let dict = py.import("dataclasses")?.call_method1("asdict", (&obj,))?;
        serialize_value(py, &dict, w, depth, default, allow_nan, sort_keys)?;
        return Ok(());
    }
    if !default.is_null() {
        let bound_default = Bound::from_borrowed_ptr(py, default);
        let result = bound_default.call1((&obj,))?;
        serialize_value(py, &result, w, depth, default, allow_nan, sort_keys)?;
        return Ok(());
    }
    Err(pyo3::exceptions::PyTypeError::new_err(format!(
        "Object of type {} is not JSON serializable",
        type_name
    )))
}

unsafe fn serialize_model(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    w: &mut JsonWriter,
    depth: usize,
    default: *mut PyObject,
    allow_nan: bool,
    sort_keys: bool,
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
        if !first {
            w.write_comma();
        }
        w.write_string(&field_name);
        w.write_colon();
        let value = obj.getattr(field_name.as_str())?;
        serialize_value(py, &value, w, depth + 1, default, allow_nan, sort_keys)?;
        first = false;
    }
    w.write_object_close();
    Ok(())
}

unsafe fn serialize_queryset(
    py: Python<'_>,
    qs: &Bound<'_, PyAny>,
    w: &mut JsonWriter,
    depth: usize,
    default: *mut PyObject,
    allow_nan: bool,
    sort_keys: bool,
) -> Result<(), PyErr> {
    let iterator = pyo3::types::PyIterator::from_object(qs)?;
    w.write_array_open();
    let mut first = true;
    for item_result in iterator {
        let item = item_result?;
        if !first {
            w.write_comma();
        }
        serialize_value(py, &item, w, depth + 1, default, allow_nan, sort_keys)?;
        first = false;
    }
    w.write_array_close();
    Ok(())
}
