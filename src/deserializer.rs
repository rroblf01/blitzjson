use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::IntoPyObjectExt;
use serde_json::Value;

pub fn deserialize<'py>(
    py: Python<'py>,
    s: &str,
    object_hook: Option<&Bound<'py, PyAny>>,
    object_pairs_hook: Option<&Bound<'py, PyAny>>,
    parse_float: Option<&Bound<'py, PyAny>>,
    parse_int: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let value: Value = serde_json::from_str(s)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    json_to_python(py, &value, object_hook, object_pairs_hook, parse_float, parse_int)
}

fn json_to_python<'py>(
    py: Python<'py>,
    value: &Value,
    object_hook: Option<&Bound<'py, PyAny>>,
    object_pairs_hook: Option<&Bound<'py, PyAny>>,
    parse_float: Option<&Bound<'py, PyAny>>,
    parse_int: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    match value {
        Value::Null => Ok(py.None().into_bound(py)),
        Value::Bool(b) => {
            let val: bool = *b;
            let obj: Bound<'py, PyAny> = val.into_py_any(py)?.into_bound(py);
            Ok(obj)
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if let Some(pf) = parse_int {
                    return pf.call1((i.to_string(),));
                }
                Ok(i.into_pyobject(py)?.into_any())
            } else if let Some(u) = n.as_u64() {
                if let Some(pf) = parse_int {
                    return pf.call1((u.to_string(),));
                }
                Ok(u.into_pyobject(py)?.into_any())
            } else if let Some(f) = n.as_f64() {
                if let Some(pf) = parse_float {
                    return pf.call1((f.to_string(),));
                }
                Ok(f.into_pyobject(py)?.into_any())
            } else {
                Ok(py.None().into_bound(py))
            }
        }
        Value::String(s) => Ok(s.into_pyobject(py)?.into_any()),
        Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                let py_item = json_to_python(py, item, object_hook, object_pairs_hook, parse_float, parse_int)?;
                list.append(py_item)?;
            }
            Ok(list.into_any())
        }
        Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                let py_val = json_to_python(py, v, object_hook, object_pairs_hook, parse_float, parse_int)?;
                dict.set_item(k.as_str(), py_val)?;
            }

            if let Some(hook) = object_pairs_hook {
                let pairs = PyList::empty(py);
                for (k, v) in dict.iter() {
                    let pair = PyList::empty(py);
                    pair.append(k)?;
                    pair.append(v)?;
                    pairs.append(pair)?;
                }
                return hook.call1((pairs,));
            }

            if let Some(hook) = object_hook {
                return hook.call1((&dict,));
            }

            Ok(dict.into_any())
        }
    }
}
