use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_polars::error::PyPolarsErr;

use crate::frame::JanitorError;

pub(super) fn map_janitor_error(error: JanitorError) -> PyErr {
    match error {
        JanitorError::Value(message) => PyValueError::new_err(message),
        JanitorError::Polars(error) => PyPolarsErr::from(error).into(),
    }
}
