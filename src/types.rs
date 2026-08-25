use pyo3::prelude::*;
use pyo3::types::{PyDateTime, PyDate, PyTime, PyDelta, PyDict, PyList, PyTuple, PySet, PyFrozenSet, PyBytes, PyByteArray, PyIterator, PyTzInfoAccess};
use chrono::{DateTime, FixedOffset, Utc, NaiveDateTime, NaiveDate, NaiveTime, Duration, Timelike};
use rust_decimal::Decimal;
use uuid::Uuid;

/// Serialize a Python object to a serde_json::Value, handling Django types natively.
pub fn python_to_json(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    if obj.is_none() {
        return Ok(serde_json::Value::Null);
    }

    // bool must be checked BEFORE int, because Python bool is a subclass of int
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(serde_json::Value::Bool(b));
    }

    // int
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(serde_json::Value::Number(i.into()));
    }

    // UUID must be checked BEFORE str (UUID has __str__)
    if let Ok(u) = obj.extract::<Uuid>() {
        return Ok(serde_json::Value::String(u.to_string()));
    }

    // Must check Python type name first: Decimal has __float__ so extract::<f64> matches it,
    // and float has __str__ so extract::<String> matches it.
    if let Ok(name) = obj.get_type().name() {
        let type_name = name.to_string_lossy();
        match type_name.as_ref() {
            "float" => {
                let f: f64 = obj.extract()?;
                if f.is_nan() || f.is_infinite() {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "Out of range float values are not JSON compliant"
                    ));
                }
                return Ok(serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null));
            }
            "Decimal" => {
                if let Ok(d) = obj.extract::<Decimal>() {
                    return Ok(serde_json::Value::String(d.to_string()));
                }
            }
            _ => {}
        }
    } else {
        // Fallback: try extract in order
        if let Ok(f) = obj.extract::<f64>() {
            if f.is_nan() || f.is_infinite() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "Out of range float values are not JSON compliant"
                ));
            }
            return Ok(serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null));
        }
        if let Ok(d) = obj.extract::<Decimal>() {
            return Ok(serde_json::Value::String(d.to_string()));
        }
    }

    // datetime types must be checked BEFORE str (they have __str__)
    if let Ok(dt) = obj.extract::<NaiveDateTime>() {
        return Ok(serde_json::Value::String(format_datetime_naive(dt)));
    }

    if let Ok(dt) = obj.extract::<DateTime<FixedOffset>>() {
        return Ok(serde_json::Value::String(format_datetime_with_tz(dt)));
    }

    if let Ok(dt) = obj.extract::<DateTime<Utc>>() {
        return Ok(serde_json::Value::String(format_datetime_utc(dt)));
    }

    if let Ok(d) = obj.extract::<NaiveDate>() {
        return Ok(serde_json::Value::String(d.format("%Y-%m-%d").to_string()));
    }

    if let Ok(t) = obj.extract::<NaiveTime>() {
        return Ok(serde_json::Value::String(format_time(t)));
    }

    if let Ok(td) = obj.extract::<Duration>() {
        return Ok(serde_json::Value::String(format_duration(td)));
    }

    // str (last of the primitive types)
    if let Ok(s) = obj.extract::<String>() {
        return Ok(serde_json::Value::String(s));
    }

    // Python datetime.datetime (aware via type checking)
    if let Ok(dt) = obj.cast::<PyDateTime>() {
        if let Some(tz) = dt.get_tzinfo() {
            if !tz.is_none() {
                let iso = dt.call_method0("isoformat")?;
                let s: String = iso.extract()?;
                return Ok(serde_json::Value::String(s));
            }
        }
        let iso = dt.call_method0("isoformat")?;
        let s: String = iso.extract()?;
        return Ok(serde_json::Value::String(s));
    }

    if let Ok(d) = obj.cast::<PyDate>() {
        let iso = d.call_method0("isoformat")?;
        let s: String = iso.extract()?;
        return Ok(serde_json::Value::String(s));
    }

    if let Ok(t) = obj.cast::<PyTime>() {
        let iso = t.call_method0("isoformat")?;
        let s: String = iso.extract()?;
        return Ok(serde_json::Value::String(s));
    }

    if let Ok(td) = obj.cast::<PyDelta>() {
        let total_seconds: f64 = td.call_method0("total_seconds")?.extract()?;
        let days: i64 = td.getattr("days")?.extract()?;
        let hours = (total_seconds / 3600.0) as i64;
        let minutes = ((total_seconds % 3600.0) / 60.0) as i64;
        let seconds = total_seconds % 60.0;
        let secs = seconds as i64;
        let micros = ((seconds - secs as f64) * 1_000_000.0) as u32;

        let mut result = String::from("P");
        if days != 0 {
            result.push_str(&format!("{}D", days));
        }
        if hours != 0 || minutes != 0 || secs != 0 || micros != 0 {
            result.push_str(&format!("T{}H{}M", hours, minutes));
            if micros > 0 {
                result.push_str(&format!("{}.{:06}S", secs, micros));
            } else {
                result.push_str(&format!("{}S", secs));
            }
        }
        if result == "P" {
            result.push_str("T0S");
        }
        return Ok(serde_json::Value::String(result));
    }

    // list / tuple
    if let Ok(list) = obj.cast::<PyList>() {
        let mut arr = Vec::with_capacity(list.len());
        for item in list.iter() {
            arr.push(python_to_json(py, &item)?);
        }
        return Ok(serde_json::Value::Array(arr));
    }

    if let Ok(tuple) = obj.cast::<PyTuple>() {
        let mut arr = Vec::with_capacity(tuple.len());
        for item in tuple.iter() {
            arr.push(python_to_json(py, &item)?);
        }
        return Ok(serde_json::Value::Array(arr));
    }

    // set / frozenset
    if let Ok(s) = obj.cast::<PySet>() {
        let mut arr: Vec<serde_json::Value> = Vec::new();
        for item in s.iter() {
            arr.push(python_to_json(py, &item)?);
        }
        arr.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
        return Ok(serde_json::Value::Array(arr));
    }

    if let Ok(fs) = obj.cast::<PyFrozenSet>() {
        let mut arr: Vec<serde_json::Value> = Vec::new();
        for item in fs.iter() {
            arr.push(python_to_json(py, &item)?);
        }
        arr.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
        return Ok(serde_json::Value::Array(arr));
    }

    // dict
    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut map = serde_json::Map::with_capacity(dict.len());
        for (k, v) in dict.iter() {
            let key: String = if let Ok(s) = k.extract::<String>() {
                s
            } else if let Ok(i) = k.extract::<i64>() {
                i.to_string()
            } else if let Ok(f) = k.extract::<f64>() {
                f.to_string()
            } else {
                k.str()?.to_string()
            };
            map.insert(key, python_to_json(py, &v)?);
        }
        return Ok(serde_json::Value::Object(map));
    }

    // bytes
    if let Ok(b) = obj.cast::<PyBytes>() {
        return Ok(serde_json::Value::String(encode_base64(b.as_bytes())));
    }

    // bytearray
    if let Ok(ba) = obj.cast::<PyByteArray>() {
        let bytes = unsafe { ba.as_bytes().to_vec() };
        return Ok(serde_json::Value::String(encode_base64(&bytes)));
    }

    // Django Promise (lazy strings)
    let is_promise = obj
        .py()
        .import("django.utils.functional")
        .and_then(|m| m.getattr("Promise"))
        .and_then(|p| obj.is_instance(&p))
        .unwrap_or(false);
    if is_promise {
        let s: String = obj.str()?.to_string();
        return Ok(serde_json::Value::String(s));
    }

    // Django Model
    let is_model = obj
        .py()
        .import("django.db.models")
        .and_then(|m| m.getattr("Model"))
        .and_then(|m| obj.is_instance(&m))
        .unwrap_or(false);
    if is_model {
        return serialize_model(py, obj);
    }

    // Django QuerySet
    let is_queryset = obj
        .py()
        .import("django.db.models.query")
        .and_then(|m| m.getattr("QuerySet"))
        .and_then(|qs| obj.is_instance(&qs))
        .unwrap_or(false);
    if is_queryset {
        return serialize_queryset_direct(py, obj);
    }

    // enum
    let is_enum = obj
        .py()
        .import("enum")
        .and_then(|m| m.getattr("Enum"))
        .and_then(|e| obj.is_instance(&e))
        .unwrap_or(false);
    if is_enum {
        let val = obj.getattr("value")?;
        return python_to_json(py, &val);
    }

    // dataclass
    if obj.hasattr("__dataclass_fields__").unwrap_or(false) {
        let dict = obj.py().import("dataclasses")?.call_method1("asdict", (obj,))?;
        return python_to_json(py, &dict);
    }

    // Fallback: try to_dict()
    if let Ok(method) = obj.getattr("to_dict") {
        if let Ok(d) = method.call0() {
            return python_to_json(py, &d);
        }
    }

    Err(pyo3::exceptions::PyTypeError::new_err(format!(
        "Object of type {} is not JSON serializable",
        obj.get_type().name()?
    )))
}

fn serialize_model(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    let meta = obj.getattr("_meta")?;
    let fields = meta.getattr("fields")?;
    let mut map = serde_json::Map::new();

    let iter = fields.call_method0("__iter__")?;
    let py_iter = PyIterator::from_object(&iter)?;
    for item_result in py_iter {
        let field = item_result?;
        let field_name: String = field.getattr("name")?.extract()?;
        let value = obj.getattr(field_name.as_str())?;
        map.insert(field_name, python_to_json(py, &value)?);
    }

    Ok(serde_json::Value::Object(map))
}

fn serialize_queryset_direct(py: Python<'_>, queryset: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    let mut arr = Vec::new();
    let iterator = PyIterator::from_object(queryset)?;
    for item_result in iterator {
        let item = item_result?;
        arr.push(python_to_json(py, &item)?);
    }
    Ok(serde_json::Value::Array(arr))
}

fn format_datetime_naive(dt: NaiveDateTime) -> String {
    if dt.and_utc().timestamp_subsec_micros() > 0 {
        dt.format("%Y-%m-%dT%H:%M:%S.%f").to_string()
    } else {
        dt.format("%Y-%m-%dT%H:%M:%S").to_string()
    }
}

fn format_datetime_with_tz(dt: DateTime<FixedOffset>) -> String {
    if dt.timestamp_subsec_micros() > 0 {
        let s = dt.format("%Y-%m-%dT%H:%M:%S.%f%:z").to_string();
        if s.ends_with("+00:00") { s.replace("+00:00", "Z") } else { s }
    } else {
        let s = dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
        if s.ends_with("+00:00") { s.replace("+00:00", "Z") } else { s }
    }
}

fn format_datetime_utc(dt: DateTime<Utc>) -> String {
    if dt.timestamp_subsec_micros() > 0 {
        dt.format("%Y-%m-%dT%H:%M:%S.%fZ").to_string()
    } else {
        dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}

fn format_time(t: NaiveTime) -> String {
    if t.nanosecond() > 0 {
        t.format("%H:%M:%S.%f").to_string()
    } else {
        t.format("%H:%M:%S").to_string()
    }
}

fn format_duration(d: Duration) -> String {
    let total_secs = d.num_seconds();
    let micros = d.subsec_nanos() as i64 / 1000;
    let days = total_secs / 86400;
    let remaining = total_secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let secs = remaining % 60;

    let mut result = String::from("P");
    if days != 0 {
        result.push_str(&format!("{}D", days));
    }
    if hours != 0 || minutes != 0 || secs != 0 || micros != 0 {
        result.push_str("T");
        if hours != 0 { result.push_str(&format!("{}H", hours)); }
        if minutes != 0 { result.push_str(&format!("{}M", minutes)); }
        if micros > 0 {
            result.push_str(&format!("{}.{:06}S", secs, micros));
        } else if secs != 0 {
            result.push_str(&format!("{}S", secs));
        }
    }
    if result == "P" { result.push_str("T0S"); }
    result
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
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}
