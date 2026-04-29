use pyo3::prelude::*;

#[pyfunction]
fn core_hello() {
    gkit_core::core_hello();
}

#[pymodule]
fn _gkit_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(core_hello, m)?)?;
    Ok(())
}
