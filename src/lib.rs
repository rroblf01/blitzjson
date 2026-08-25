mod deserializer;
mod ffi_serializer;
mod writer;

use pyo3::prelude::*;
use writer::JsonWriter;

// ═══════════════════════════════════════════════════════════════════
// dumps - Drop-in replacement for json.dumps()
// ═══════════════════════════════════════════════════════════════════

#[allow(unused_variables)]
#[allow(clippy::too_many_arguments)]
#[pyfunction]
#[pyo3(signature = (obj, skipkeys=false, ensure_ascii=true, check_circular=true, allow_nan=true, cls=None, indent=None, separators=None, default=None, sort_keys=false, **_kw))]
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
    _kw: Option<&Bound<'py, PyAny>>,
) -> PyResult<String> {
    let indent_val = indent.and_then(|i| i.extract::<u8>().ok());
    let (item_sep, key_sep): (&'static str, &'static str) = match separators {
        Some(seps) => {
            let item: String = seps.get_item(0)?.extract()?;
            let key: String = seps.get_item(1)?.extract()?;
            (
                Box::leak(item.into_boxed_str()),
                Box::leak(key.into_boxed_str()),
            )
        }
        None => {
            // json.dumps uses compact item separator when indent is set,
            // but key separator is always ": " (with space)
            if indent_val.is_some() {
                (",", ": ")
            } else {
                (", ", ": ")
            }
        }
    };
    let mut writer =
        JsonWriter::with_separators(1024, ensure_ascii, indent_val, sort_keys, item_sep, key_sep);
    let default_obj = default.map(|d| d.as_ptr()).unwrap_or(std::ptr::null_mut());

    let result = unsafe {
        ffi_serializer::ffi_serialize(py, obj.as_ptr(), &mut writer, 0, default_obj, allow_nan)
    };
    match result {
        Ok(()) => Ok(writer.into_string()),
        Err(e) => {
            if let Some(fallback) = default {
                let result = fallback.call1((obj,))?;
                let mut writer2 =
                    JsonWriter::with_options(1024, ensure_ascii, indent_val, sort_keys);
                unsafe {
                    ffi_serializer::ffi_serialize(
                        py,
                        result.as_ptr(),
                        &mut writer2,
                        0,
                        default_obj,
                        allow_nan,
                    )?;
                }
                Ok(writer2.into_string())
            } else {
                Err(e)
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// loads - Drop-in replacement for json.loads()
// ═══════════════════════════════════════════════════════════════════

#[allow(unused_variables)]
#[allow(clippy::too_many_arguments)]
#[pyfunction]
#[pyo3(signature = (s, cls=None, object_hook=None, object_pairs_hook=None, parse_float=None, parse_int=None, parse_constant=None, strict=true, **_kw))]
fn loads<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    cls: Option<&Bound<'py, PyAny>>,
    object_hook: Option<&Bound<'py, PyAny>>,
    object_pairs_hook: Option<&Bound<'py, PyAny>>,
    parse_float: Option<&Bound<'py, PyAny>>,
    parse_int: Option<&Bound<'py, PyAny>>,
    parse_constant: Option<&Bound<'py, PyAny>>,
    strict: bool,
    _kw: Option<&Bound<'py, PyAny>>,
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
    deserializer::deserialize_direct(
        py,
        &json_str,
        object_hook,
        object_pairs_hook,
        parse_float,
        parse_int,
    )
}

// ═══════════════════════════════════════════════════════════════════
// dump - Drop-in replacement for json.dump()
// ═══════════════════════════════════════════════════════════════════

#[allow(unused_variables)]
#[allow(clippy::too_many_arguments)]
#[pyfunction]
#[pyo3(signature = (obj, fp, skipkeys=false, ensure_ascii=true, check_circular=true, allow_nan=true, cls=None, indent=None, separators=None, default=None, sort_keys=false, **_kw))]
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
    _kw: Option<&Bound<'py, PyAny>>,
) -> PyResult<()> {
    let s = dumps(
        py,
        obj,
        skipkeys,
        ensure_ascii,
        check_circular,
        allow_nan,
        cls,
        indent,
        separators,
        default,
        sort_keys,
        None,
    )?;
    fp.call_method1("write", (&s,))?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// load - Drop-in replacement for json.load()
// ═══════════════════════════════════════════════════════════════════

#[allow(unused_variables)]
#[allow(clippy::too_many_arguments)]
#[pyfunction]
#[pyo3(signature = (fp, cls=None, object_hook=None, object_pairs_hook=None, parse_float=None, parse_int=None, parse_constant=None, strict=true, **_kw))]
fn load<'py>(
    py: Python<'py>,
    fp: &Bound<'py, PyAny>,
    cls: Option<&Bound<'py, PyAny>>,
    object_hook: Option<&Bound<'py, PyAny>>,
    object_pairs_hook: Option<&Bound<'py, PyAny>>,
    parse_float: Option<&Bound<'py, PyAny>>,
    parse_int: Option<&Bound<'py, PyAny>>,
    parse_constant: Option<&Bound<'py, PyAny>>,
    strict: bool,
    _kw: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let content: String = fp.call_method0("read")?.extract()?;
    let json_str = content.trim();
    deserializer::deserialize_direct(
        py,
        json_str,
        object_hook,
        object_pairs_hook,
        parse_float,
        parse_int,
    )
}

// ═══════════════════════════════════════════════════════════════════
// Extra functions (blitzjson extensions)
// ═══════════════════════════════════════════════════════════════════

#[pyfunction]
#[pyo3(signature = (obj, pretty=false))]
fn dumpb(py: Python<'_>, obj: &Bound<'_, PyAny>, pretty: bool) -> PyResult<Vec<u8>> {
    let indent = if pretty { Some(2) } else { None };
    let mut writer = JsonWriter::with_options(1024, false, indent, false);
    unsafe {
        ffi_serializer::ffi_serialize(
            py,
            obj.as_ptr(),
            &mut writer,
            0,
            std::ptr::null_mut(),
            true,
        )?;
    }
    Ok(writer.into_bytes())
}

#[pyfunction]
fn dump_queryset(py: Python<'_>, queryset: &Bound<'_, PyAny>) -> PyResult<String> {
    let mut writer = JsonWriter::new(1024);
    unsafe {
        ffi_serializer::ffi_serialize(
            py,
            queryset.as_ptr(),
            &mut writer,
            0,
            std::ptr::null_mut(),
            true,
        )?;
    }
    Ok(writer.into_string())
}

#[pyfunction]
fn dump_queryset_bytes(py: Python<'_>, queryset: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    let mut writer = JsonWriter::new(1024);
    unsafe {
        ffi_serializer::ffi_serialize(
            py,
            queryset.as_ptr(),
            &mut writer,
            0,
            std::ptr::null_mut(),
            true,
        )?;
    }
    Ok(writer.into_bytes())
}

#[pyfunction]
#[pyo3(signature = (queryset, fp, chunk_size=1000))]
fn stream_dump_queryset(
    py: Python<'_>,
    queryset: &Bound<'_, PyAny>,
    fp: &Bound<'_, PyAny>,
    chunk_size: usize,
) -> PyResult<()> {
    let iterator = pyo3::types::PyIterator::from_object(queryset)?;
    let mut writer = JsonWriter::new(chunk_size * 128);
    let mut first = true;
    writer.write_array_open();
    for item_result in iterator {
        let item = item_result?;
        if !first {
            writer.buf_mut().push(',');
        }
        unsafe {
            ffi_serializer::ffi_serialize(
                py,
                item.as_ptr(),
                &mut writer,
                0,
                std::ptr::null_mut(),
                true,
            )?;
        }
        first = false;
        if writer.buf_mut().len() > chunk_size * 128 {
            fp.call_method1("write", (writer.buf_mut().as_str(),))?;
            writer.buf_mut().clear();
        }
    }
    writer.write_array_close();
    if !writer.buf_mut().is_empty() {
        fp.call_method1("write", (writer.buf_mut().as_str(),))?;
    }
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (queryset, fp, chunk_size=1000))]
fn stream_dump_queryset_jsonl(
    py: Python<'_>,
    queryset: &Bound<'_, PyAny>,
    fp: &Bound<'_, PyAny>,
    chunk_size: usize,
) -> PyResult<()> {
    let iterator = pyo3::types::PyIterator::from_object(queryset)?;
    let mut writer = JsonWriter::new(chunk_size * 128);
    for item_result in iterator {
        let item = item_result?;
        unsafe {
            ffi_serializer::ffi_serialize(
                py,
                item.as_ptr(),
                &mut writer,
                0,
                std::ptr::null_mut(),
                true,
            )?;
        }
        writer.buf_mut().push('\n');
        if writer.buf_mut().len() > chunk_size * 128 {
            fp.call_method1("write", (writer.buf_mut().as_str(),))?;
            writer.buf_mut().clear();
        }
    }
    if !writer.buf_mut().is_empty() {
        fp.call_method1("write", (writer.buf_mut().as_str(),))?;
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// Module definition
// ═══════════════════════════════════════════════════════════════════

#[pyfunction]
#[pyo3(signature = (s, object_hook=None, object_pairs_hook=None, parse_float=None, parse_int=None))]
fn deserialize_direct_fn<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    object_hook: Option<&Bound<'py, PyAny>>,
    object_pairs_hook: Option<&Bound<'py, PyAny>>,
    parse_float: Option<&Bound<'py, PyAny>>,
    parse_int: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let json_str: String = s.extract()?;
    deserializer::deserialize_direct(
        py,
        &json_str,
        object_hook,
        object_pairs_hook,
        parse_float,
        parse_int,
    )
}

#[pyfunction]
#[pyo3(signature = (s, object_hook=None, object_pairs_hook=None, parse_float=None, parse_int=None))]
fn deserialize_strict_fn<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    object_hook: Option<&Bound<'py, PyAny>>,
    object_pairs_hook: Option<&Bound<'py, PyAny>>,
    parse_float: Option<&Bound<'py, PyAny>>,
    parse_int: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let json_str: String = s.extract()?;
    deserializer::deserialize_strict(
        py,
        &json_str,
        object_hook,
        object_pairs_hook,
        parse_float,
        parse_int,
    )
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
    m.add_function(wrap_pyfunction!(stream_dump_queryset, m)?)?;
    m.add_function(wrap_pyfunction!(stream_dump_queryset_jsonl, m)?)?;
    m.add_function(wrap_pyfunction!(deserialize_direct_fn, m)?)?;
    m.add_function(wrap_pyfunction!(deserialize_strict_fn, m)?)?;
    m.add("__version__", "0.1.0")?;
    Ok(())
}
