mod ffi_serializer;
mod writer;
mod deserializer;

use pyo3::prelude::*;
use writer::JsonWriter;

#[allow(unused_variables)]
#[pyfunction]
#[pyo3(signature = (obj, skipkeys=false, ensure_ascii=true, check_circular=true, allow_nan=true, cls=None, indent=None, separators=None, default=None, sort_keys=false))]
fn dumps<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    skipkeys: bool,
    ensure_ascii: bool,
    check_circular: bool,
    allow_nan: bool,
    cls: Option<&Bound<'py, PyAny>>,
    indent: Option<&Bound<'py, PyAny>>,
    separators: Option<&Bound<'py, PyAny>>,
    default: Option<&Bound<'py, PyAny>>,
    sort_keys: bool,
) -> PyResult<String> {
    let mut writer = JsonWriter::new(1024);
    let result = unsafe {
        ffi_serializer::ffi_serialize(py, obj.as_ptr(), &mut writer, 0)
    };
    match result {
        Ok(()) => Ok(writer.into_string()),
        Err(e) => {
            if let Some(fallback) = default {
                let result = fallback.call1((obj,))?;
                let mut writer2 = JsonWriter::new(1024);
                unsafe {
                    ffi_serializer::ffi_serialize(py, result.as_ptr(), &mut writer2, 0)?;
                }
                Ok(writer2.into_string())
            } else {
                Err(e)
            }
        }
    }
}

#[allow(unused_variables)]
#[pyfunction]
#[pyo3(signature = (s, cls=None, object_hook=None, object_pairs_hook=None, parse_float=None, parse_int=None, parse_constant=None, object_pairs_pairs_hook=None))]
fn loads<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    cls: Option<&Bound<'py, PyAny>>,
    object_hook: Option<&Bound<'py, PyAny>>,
    object_pairs_hook: Option<&Bound<'py, PyAny>>,
    parse_float: Option<&Bound<'py, PyAny>>,
    parse_int: Option<&Bound<'py, PyAny>>,
    parse_constant: Option<&Bound<'py, PyAny>>,
    object_pairs_pairs_hook: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let json_str: String = if s.is_instance_of::<pyo3::types::PyBytes>() {
        let bytes: &[u8] = s.extract()?;
        String::from_utf8_lossy(bytes).to_string()
    } else if s.is_instance_of::<pyo3::types::PyByteArray>() {
        let bytes: Vec<u8> = s.extract()?;
        String::from_utf8_lossy(&bytes).to_string()
    } else {
        s.extract()?
    };

    deserializer::deserialize_direct(py, &json_str, object_hook, object_pairs_hook, parse_float, parse_int)
}

#[allow(unused_variables)]
#[pyfunction]
#[pyo3(signature = (obj, fp, skipkeys=false, ensure_ascii=true, check_circular=true, allow_nan=true, cls=None, indent=None, separators=None, default=None, sort_keys=false))]
fn dump<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    fp: &Bound<'py, PyAny>,
    skipkeys: bool,
    ensure_ascii: bool,
    check_circular: bool,
    allow_nan: bool,
    cls: Option<&Bound<'py, PyAny>>,
    indent: Option<&Bound<'py, PyAny>>,
    separators: Option<&Bound<'py, PyAny>>,
    default: Option<&Bound<'py, PyAny>>,
    sort_keys: bool,
) -> PyResult<()> {
    let s = dumps(py, obj, skipkeys, ensure_ascii, check_circular, allow_nan, cls, indent, separators, default, sort_keys)?;
    fp.call_method1("write", (&s,))?;
    Ok(())
}

#[allow(unused_variables)]
#[pyfunction]
#[pyo3(signature = (fp, cls=None, object_hook=None, object_pairs_hook=None, parse_float=None, parse_int=None, parse_constant=None))]
fn load<'py>(
    py: Python<'py>,
    fp: &Bound<'py, PyAny>,
    cls: Option<&Bound<'py, PyAny>>,
    object_hook: Option<&Bound<'py, PyAny>>,
    object_pairs_hook: Option<&Bound<'py, PyAny>>,
    parse_float: Option<&Bound<'py, PyAny>>,
    parse_int: Option<&Bound<'py, PyAny>>,
    parse_constant: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let content: String = fp.call_method0("read")?.extract()?;
    let json_str = content.trim();
    deserializer::deserialize_direct(py, json_str, object_hook, object_pairs_hook, parse_float, parse_int)
}

#[pyfunction]
#[pyo3(signature = (obj, _pretty=false))]
fn dumpb(py: Python<'_>, obj: &Bound<'_, PyAny>, _pretty: bool) -> PyResult<Vec<u8>> {
    let mut writer = JsonWriter::new(1024);
    unsafe {
        ffi_serializer::ffi_serialize(py, obj.as_ptr(), &mut writer, 0)?;
    }
    Ok(writer.into_bytes())
}

#[pyfunction]
fn dump_queryset(py: Python<'_>, queryset: &Bound<'_, PyAny>) -> PyResult<String> {
    let mut writer = JsonWriter::new(1024);
    unsafe {
        ffi_serializer::ffi_serialize(py, queryset.as_ptr(), &mut writer, 0)?;
    }
    Ok(writer.into_string())
}

#[pyfunction]
fn dump_queryset_bytes(py: Python<'_>, queryset: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    let mut writer = JsonWriter::new(1024);
    unsafe {
        ffi_serializer::ffi_serialize(py, queryset.as_ptr(), &mut writer, 0)?;
    }
    Ok(writer.into_bytes())
}

#[pymodule]
#[pyo3(name = "_core")]
fn blitzjson(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(dumps, m)?)?;
    m.add_function(wrap_pyfunction!(loads, m)?)?;
    m.add_function(wrap_pyfunction!(dump, m)?)?;
    m.add_function(wrap_pyfunction!(load, m)?)?;
    m.add_function(wrap_pyfunction!(dumpb, m)?)?;
    m.add_function(wrap_pyfunction!(dump_queryset, m)?)?;
    m.add_function(wrap_pyfunction!(dump_queryset_bytes, m)?)?;
    m.add("__version__", "0.1.0")?;
    Ok(())
}
