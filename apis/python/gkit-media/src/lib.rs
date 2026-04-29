use pyo3::prelude::*;

#[pyfunction]
fn media_hello() {
    gkit_media::media_hello();
}

#[pymodule]
fn _gkit_media(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(media_hello, m)?)?;
    Ok(())
}
