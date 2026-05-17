mod frame;
mod names;
mod python;

use pyo3::prelude::*;
#[cfg(not(test))]
use pyo3_polars::PolarsAllocator;

#[cfg(not(test))]
#[global_allocator]
static ALLOC: PolarsAllocator = PolarsAllocator::new();

#[pymodule]
fn _rust(module: &Bound<'_, PyModule>) -> PyResult<()> {
    python::register(module)
}
