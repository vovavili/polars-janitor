mod args;
mod errors;
mod lazy;

use polars::prelude::*;
use pyo3::exceptions::{PyNotImplementedError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3_polars::PyDataFrame;

use crate::frame;
use crate::names;

use self::args::{
    is_polars_class, parse_optional_columns, parse_required_columns, stringify_iterable,
};
use self::errors::map_janitor_error;
use self::lazy::{clean_names_lazy_py, get_dupes_lazy_py, remove_empty_lazy_py};

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(make_clean_names, module)?)?;
    module.add_function(wrap_pyfunction!(clean_names, module)?)?;
    module.add_function(wrap_pyfunction!(remove_empty, module)?)?;
    module.add_function(wrap_pyfunction!(remove_constant, module)?)?;
    module.add_function(wrap_pyfunction!(get_dupes, module)?)?;
    Ok(())
}

#[pyfunction(signature = (names, case = "snake"))]
fn make_clean_names(names: &Bound<'_, PyAny>, case: &str) -> PyResult<Vec<String>> {
    let names = stringify_iterable(names, "names")?;
    names::make_clean_names(&names, case)
        .map_err(|error| PyValueError::new_err(error.message().to_string()))
}

#[pyfunction(signature = (frame, *, case = "snake"))]
fn clean_names(py: Python<'_>, frame: &Bound<'_, PyAny>, case: &str) -> PyResult<Py<PyAny>> {
    if is_polars_class(py, frame, "DataFrame")? {
        let df: DataFrame = frame.extract::<PyDataFrame>()?.into();
        return frame::clean_names_df(df, case)
            .map(PyDataFrame)
            .map_err(map_janitor_error)?
            .into_pyobject(py)
            .map(Bound::unbind);
    }
    if is_polars_class(py, frame, "LazyFrame")? {
        return clean_names_lazy_py(py, frame, case);
    }
    Err(PyTypeError::new_err(
        "frame must be a polars DataFrame or LazyFrame",
    ))
}

#[pyfunction(signature = (frame, *, axis = "rows", subset = None))]
fn remove_empty(
    py: Python<'_>,
    frame: &Bound<'_, PyAny>,
    axis: &str,
    subset: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    if is_polars_class(py, frame, "DataFrame")? {
        let subset = parse_optional_columns(subset, "subset")?;
        let df: DataFrame = frame.extract::<PyDataFrame>()?.into();
        return frame::remove_empty_df(df, axis, subset)
            .map(PyDataFrame)
            .map_err(map_janitor_error)?
            .into_pyobject(py)
            .map(Bound::unbind);
    }
    if is_polars_class(py, frame, "LazyFrame")? {
        return remove_empty_lazy_py(py, frame, axis, subset);
    }
    Err(PyTypeError::new_err(
        "frame must be a polars DataFrame or LazyFrame",
    ))
}

#[pyfunction(signature = (df, *, subset = None, ignore_nulls = false))]
fn remove_constant(
    py: Python<'_>,
    df: &Bound<'_, PyAny>,
    subset: Option<&Bound<'_, PyAny>>,
    ignore_nulls: bool,
) -> PyResult<Py<PyAny>> {
    if is_polars_class(py, df, "LazyFrame")? {
        return Err(PyNotImplementedError::new_err(
            "remove_constant() is data-dependent and is only supported for eager DataFrame",
        ));
    }
    if !is_polars_class(py, df, "DataFrame")? {
        return Err(PyTypeError::new_err("df must be a polars DataFrame"));
    }

    let subset = parse_optional_columns(subset, "subset")?;
    let df: DataFrame = df.extract::<PyDataFrame>()?.into();
    frame::remove_constant_df(df, subset, ignore_nulls)
        .map(PyDataFrame)
        .map_err(map_janitor_error)?
        .into_pyobject(py)
        .map(Bound::unbind)
}

#[pyfunction(signature = (frame, keys, *, include_count = true, count_name = "duplicate_count"))]
fn get_dupes(
    py: Python<'_>,
    frame: &Bound<'_, PyAny>,
    keys: &Bound<'_, PyAny>,
    include_count: bool,
    count_name: &str,
) -> PyResult<Py<PyAny>> {
    let keys = parse_required_columns(keys, "keys")?;
    if is_polars_class(py, frame, "DataFrame")? {
        let df: DataFrame = frame.extract::<PyDataFrame>()?.into();
        return frame::get_dupes_df(df, keys, include_count, count_name)
            .map(PyDataFrame)
            .map_err(map_janitor_error)?
            .into_pyobject(py)
            .map(Bound::unbind);
    }
    if is_polars_class(py, frame, "LazyFrame")? {
        return get_dupes_lazy_py(py, frame, keys, include_count, count_name);
    }
    Err(PyTypeError::new_err(
        "frame must be a polars DataFrame or LazyFrame",
    ))
}
