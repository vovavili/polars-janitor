use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyString;

pub(super) fn stringify_iterable(obj: &Bound<'_, PyAny>, label: &str) -> PyResult<Vec<String>> {
    let iterator = obj
        .try_iter()
        .map_err(|_| PyTypeError::new_err(format!("{label} must be an iterable")))?;
    iterator
        .map(|item| {
            let item = item?;
            if item.is_none() {
                Ok(String::new())
            } else {
                Ok(item.str()?.to_string())
            }
        })
        .collect()
}

pub(super) fn python_lazy_schema_names(frame: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    let names = frame
        .call_method0("collect_schema")?
        .call_method0("names")?;
    extract_string_sequence(&names, "schema")
}

pub(super) fn parse_optional_columns(
    obj: Option<&Bound<'_, PyAny>>,
    label: &str,
) -> PyResult<Option<Vec<String>>> {
    match obj {
        None => Ok(None),
        Some(value) if value.is_none() => Ok(None),
        Some(value) => parse_required_columns(value, label).map(Some),
    }
}

pub(super) fn parse_required_columns(obj: &Bound<'_, PyAny>, label: &str) -> PyResult<Vec<String>> {
    if obj.is_instance_of::<PyString>() {
        return Ok(vec![obj.extract::<String>()?]);
    }

    let iterator = obj
        .try_iter()
        .map_err(|_| PyTypeError::new_err(format!("{label} must contain only column names")))?;
    let columns = iterator
        .map(|item| {
            let item = item?;
            if item.is_instance_of::<PyString>() {
                item.extract::<String>()
            } else {
                Err(PyTypeError::new_err(format!(
                    "{label} must contain only column names"
                )))
            }
        })
        .collect::<PyResult<Vec<_>>>()?;

    if columns.is_empty() {
        Err(PyValueError::new_err(format!(
            "{label} must contain at least one column"
        )))
    } else {
        Ok(columns)
    }
}

pub(super) fn selected_columns_py(
    available: &[String],
    subset: Option<Vec<String>>,
    label: &str,
) -> PyResult<Vec<String>> {
    let selected = subset.unwrap_or_else(|| available.to_vec());
    let missing = selected
        .iter()
        .filter(|column| !available.contains(column))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(selected)
    } else {
        Err(PyValueError::new_err(format!(
            "{label} columns are not present in the frame: {}",
            python_list(&missing)
        )))
    }
}

pub(super) fn validate_key_columns_py(available: &[String], keys: &[String]) -> PyResult<()> {
    let missing = keys
        .iter()
        .filter(|key| !available.contains(key))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(PyValueError::new_err(format!(
            "keys are not present in the frame: {}",
            python_list(&missing)
        )))
    }
}

pub(super) fn is_polars_class(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    class_name: &str,
) -> PyResult<bool> {
    let polars = PyModule::import(py, "polars")?;
    obj.is_instance(&polars.getattr(class_name)?)
}

fn extract_string_sequence(obj: &Bound<'_, PyAny>, label: &str) -> PyResult<Vec<String>> {
    obj.try_iter()
        .map_err(|_| PyTypeError::new_err(format!("{label} must contain only column names")))?
        .map(|item| {
            let item = item?;
            if item.is_instance_of::<PyString>() {
                item.extract::<String>()
            } else {
                Err(PyTypeError::new_err(format!(
                    "{label} must contain only column names"
                )))
            }
        })
        .collect()
}

fn python_list(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format!("'{value}'"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
