mod args;
mod errors;
mod lazy;

use polars::prelude::*;
use pyo3::exceptions::{PyNotImplementedError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyString};
use pyo3_polars::PyDataFrame;

use crate::frame;
use crate::frame::{ColumnSchema, FrameSchema, HeaderSearchColumn};
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
    module.add_function(wrap_pyfunction!(find_header, module)?)?;
    module.add_function(wrap_pyfunction!(row_to_names, module)?)?;
    module.add_function(wrap_pyfunction!(compare_df_cols, module)?)?;
    module.add_function(wrap_pyfunction!(compare_df_cols_same, module)?)?;
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

#[pyfunction(signature = (df, *, value = None, column = None))]
fn find_header(
    py: Python<'_>,
    df: &Bound<'_, PyAny>,
    value: Option<&Bound<'_, PyAny>>,
    column: Option<&Bound<'_, PyAny>>,
) -> PyResult<usize> {
    if is_polars_class(py, df, "LazyFrame")? {
        return Err(PyNotImplementedError::new_err(
            "find_header() is data-dependent and is only supported for eager DataFrame",
        ));
    }
    if !is_polars_class(py, df, "DataFrame")? {
        return Err(PyTypeError::new_err("df must be a polars DataFrame"));
    }

    let value = stringify_optional(value)?;
    let column = parse_header_search_column(column)?;
    let df: DataFrame = df.extract::<PyDataFrame>()?.into();
    frame::find_header_df(&df, value, column).map_err(map_janitor_error)
}

#[pyfunction(signature = (df, row = None, *, remove_row = true, remove_rows_above = true, case = "snake"))]
fn row_to_names(
    py: Python<'_>,
    df: &Bound<'_, PyAny>,
    row: Option<&Bound<'_, PyAny>>,
    remove_row: bool,
    remove_rows_above: bool,
    case: &str,
) -> PyResult<Py<PyAny>> {
    if is_polars_class(py, df, "LazyFrame")? {
        return Err(PyNotImplementedError::new_err(
            "row_to_names() is data-dependent and is only supported for eager DataFrame",
        ));
    }
    if !is_polars_class(py, df, "DataFrame")? {
        return Err(PyTypeError::new_err("df must be a polars DataFrame"));
    }

    let row = parse_row_selector(row)?;
    let df: DataFrame = df.extract::<PyDataFrame>()?.into();
    let row_index = match row {
        RowSelector::Index(index) => index,
        RowSelector::FindHeader => {
            frame::find_header_df(&df, None, None).map_err(map_janitor_error)?
        }
    };
    frame::row_to_names_df(df, row_index, remove_row, remove_rows_above, case)
        .map(PyDataFrame)
        .map_err(map_janitor_error)?
        .into_pyobject(py)
        .map(Bound::unbind)
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

#[pyfunction(signature = (frames, *, names = None, return_ = "all"))]
fn compare_df_cols(
    py: Python<'_>,
    frames: &Bound<'_, PyAny>,
    names: Option<&Bound<'_, PyAny>>,
    return_: &str,
) -> PyResult<Py<PyAny>> {
    let schemas = parse_frame_schemas(py, frames, names)?;
    frame::compare_df_cols_schemas(&schemas, return_)
        .map(PyDataFrame)
        .map_err(map_janitor_error)?
        .into_pyobject(py)
        .map(Bound::unbind)
}

#[pyfunction(signature = (frames, *, names = None))]
fn compare_df_cols_same(
    py: Python<'_>,
    frames: &Bound<'_, PyAny>,
    names: Option<&Bound<'_, PyAny>>,
) -> PyResult<bool> {
    let schemas = parse_frame_schemas(py, frames, names)?;
    frame::compare_df_cols_same_schemas(&schemas).map_err(map_janitor_error)
}

enum RowSelector {
    FindHeader,
    Index(usize),
}

fn stringify_optional(obj: Option<&Bound<'_, PyAny>>) -> PyResult<Option<String>> {
    match obj {
        None => Ok(None),
        Some(value) if value.is_none() => Ok(None),
        Some(value) => Ok(Some(value.str()?.to_string())),
    }
}

fn parse_row_selector(obj: Option<&Bound<'_, PyAny>>) -> PyResult<RowSelector> {
    let Some(value) = obj else {
        return Ok(RowSelector::FindHeader);
    };
    if value.is_none() {
        return Ok(RowSelector::FindHeader);
    }
    if value.is_instance_of::<PyString>() {
        let value = value.extract::<String>()?;
        if value == "find_header" {
            return Ok(RowSelector::FindHeader);
        }
        return Err(PyValueError::new_err(
            "row must be a non-negative integer, None, or 'find_header'",
        ));
    }
    if value.is_instance_of::<PyBool>() {
        return Err(PyTypeError::new_err(
            "row must be a non-negative integer, None, or 'find_header'",
        ));
    }

    let index = value.extract::<i64>().map_err(|_| {
        PyTypeError::new_err("row must be a non-negative integer, None, or 'find_header'")
    })?;
    usize::try_from(index).map(RowSelector::Index).map_err(|_| {
        PyValueError::new_err("row must be a non-negative integer, None, or 'find_header'")
    })
}

fn parse_header_search_column(
    obj: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<HeaderSearchColumn>> {
    let Some(value) = obj else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    if value.is_instance_of::<PyString>() {
        return Ok(Some(HeaderSearchColumn::Name(value.extract::<String>()?)));
    }
    if value.is_instance_of::<PyBool>() {
        return Err(PyTypeError::new_err(
            "column must be a column name, a non-negative column index, or None",
        ));
    }

    let index = value.extract::<i64>().map_err(|_| {
        PyTypeError::new_err("column must be a column name, a non-negative column index, or None")
    })?;
    usize::try_from(index)
        .map(HeaderSearchColumn::Index)
        .map(Some)
        .map_err(|_| {
            PyValueError::new_err(
                "column must be a column name, a non-negative column index, or None",
            )
        })
}

fn parse_frame_schemas(
    py: Python<'_>,
    frames: &Bound<'_, PyAny>,
    names: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<FrameSchema>> {
    if is_polars_class(py, frames, "DataFrame")? || is_polars_class(py, frames, "LazyFrame")? {
        return Err(PyTypeError::new_err(
            "frames must be a mapping or iterable of Polars DataFrame/LazyFrame objects",
        ));
    }

    if frames.hasattr("items")? {
        if names.is_some_and(|value| !value.is_none()) {
            return Err(PyValueError::new_err(
                "names cannot be used when frames is a mapping",
            ));
        }
        let items = frames.call_method0("items")?;
        return items
            .try_iter()?
            .map(|item| {
                let item = item?;
                let name = item.get_item(0)?.str()?.to_string();
                let frame = item.get_item(1)?;
                schema_from_python_frame(py, &frame, name)
            })
            .collect();
    }

    let names = parse_optional_columns(names, "names")?;
    let mut schemas = Vec::new();
    for (index, frame) in frames
        .try_iter()
        .map_err(|_| {
            PyTypeError::new_err(
                "frames must be a mapping or iterable of Polars DataFrame/LazyFrame objects",
            )
        })?
        .enumerate()
    {
        let frame = frame?;
        let name = names
            .as_ref()
            .and_then(|names| names.get(index))
            .cloned()
            .unwrap_or_else(|| format!("frame_{}", index + 1));
        schemas.push(schema_from_python_frame(py, &frame, name)?);
    }

    if let Some(names) = names {
        if names.len() != schemas.len() {
            return Err(PyValueError::new_err(format!(
                "names must contain exactly {} values",
                schemas.len()
            )));
        }
    }

    Ok(schemas)
}

fn schema_from_python_frame(
    py: Python<'_>,
    frame: &Bound<'_, PyAny>,
    name: String,
) -> PyResult<FrameSchema> {
    if is_polars_class(py, frame, "DataFrame")? {
        let df: DataFrame = frame.extract::<PyDataFrame>()?.into();
        return Ok(frame::frame_schema_from_dataframe(name, &df));
    }
    if is_polars_class(py, frame, "LazyFrame")? {
        return Ok(FrameSchema {
            name,
            columns: schema_columns_from_lazy_frame(frame)?,
        });
    }
    Err(PyTypeError::new_err(
        "frames must contain only Polars DataFrame/LazyFrame objects",
    ))
}

fn schema_columns_from_lazy_frame(frame: &Bound<'_, PyAny>) -> PyResult<Vec<ColumnSchema>> {
    frame
        .call_method0("collect_schema")?
        .call_method0("items")?
        .try_iter()?
        .map(|item| {
            let item = item?;
            Ok(ColumnSchema {
                name: item.get_item(0)?.extract::<String>()?,
                dtype: item.get_item(1)?.str()?.to_string(),
            })
        })
        .collect()
}
