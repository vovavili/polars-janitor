use pyo3::exceptions::{PyNotImplementedError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::names;

use super::args::{
    parse_optional_columns, python_lazy_schema_names, selected_columns_py, validate_key_columns_py,
};

pub(super) fn clean_names_lazy_py(
    py: Python<'_>,
    frame: &Bound<'_, PyAny>,
    case: &str,
) -> PyResult<Py<PyAny>> {
    let names = python_lazy_schema_names(frame)?;
    let cleaned = names::make_clean_names(&names, case)
        .map_err(|error| PyValueError::new_err(error.message().to_string()))?;
    let mapping = PyDict::new(py);
    for (name, cleaned) in names.iter().zip(cleaned) {
        mapping.set_item(name, cleaned)?;
    }
    frame.call_method1("rename", (mapping,)).map(Bound::unbind)
}

pub(super) fn remove_empty_lazy_py(
    py: Python<'_>,
    frame: &Bound<'_, PyAny>,
    axis: &str,
    subset: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    match axis {
        "rows" => {}
        "cols" | "both" => {
            return Err(PyNotImplementedError::new_err(
                "remove_empty(..., axis='cols' or 'both') is data-dependent and is only supported for eager DataFrame",
            ));
        }
        _ => {
            return Err(PyValueError::new_err(
                "axis must be one of: 'rows', 'cols', 'both'",
            ));
        }
    }

    let available = python_lazy_schema_names(frame)?;
    let subset = parse_optional_columns(subset, "subset")?;
    let columns = selected_columns_py(&available, subset, "subset")?;
    if columns.is_empty() {
        return Ok(frame.clone().unbind());
    }

    let polars = PyModule::import(py, "polars")?;
    let mut expressions = Vec::with_capacity(columns.len());
    for column in columns {
        expressions.push(
            polars
                .call_method1("col", (column.as_str(),))?
                .call_method0("is_not_null")?
                .unbind(),
        );
    }
    let predicate = polars.call_method1("any_horizontal", (PyList::new(py, expressions)?,))?;
    frame
        .call_method1("filter", (predicate,))
        .map(Bound::unbind)
}

pub(super) fn get_dupes_lazy_py(
    py: Python<'_>,
    frame: &Bound<'_, PyAny>,
    keys: Vec<String>,
    include_count: bool,
    count_name: &str,
) -> PyResult<Py<PyAny>> {
    let columns = python_lazy_schema_names(frame)?;
    validate_key_columns_py(&columns, &keys)?;
    if include_count && columns.iter().any(|column| column == count_name) {
        return Err(PyValueError::new_err(format!(
            "count_name already exists in the frame: {count_name:?}"
        )));
    }

    let polars = PyModule::import(py, "polars")?;
    let key_list = PyList::new(py, keys.iter().map(String::as_str))?;
    let count_expr = polars
        .call_method0("len")?
        .call_method1("over", (key_list,))?
        .call_method1("alias", (count_name,))?;
    let with_counts = frame.call_method1("with_columns", (count_expr,))?;
    let predicate = polars
        .call_method1("col", (count_name,))?
        .call_method1("__gt__", (1,))?;
    let filtered = with_counts.call_method1("filter", (predicate,))?;

    if include_count {
        Ok(filtered.unbind())
    } else {
        filtered
            .call_method1(
                "select",
                (PyList::new(py, columns.iter().map(String::as_str))?,),
            )
            .map(Bound::unbind)
    }
}
