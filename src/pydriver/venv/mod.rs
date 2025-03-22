use eyre::Result;
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::PyString;

pub trait TracebackError<T> {
    fn format_traceback(self, py: Python<'_>) -> eyre::Result<T>;
}

impl<T> TracebackError<T> for PyResult<T> {
    fn format_traceback(self, py: Python<'_>) -> eyre::Result<T> {
        self.map_err(|e| match e.traceback(py) {
            Some(traceback) => match traceback.format() {
                Ok(traceback) => eyre::eyre!("{traceback} {e}"),
                Err(format_e) => eyre::eyre!("{e} {format_e}"),
            },
            None => eyre::eyre!("{e}"),
        })
    }
}

#[pyfunction]
pub fn info(message: String) -> PyResult<()> {
    println!("INFO: {message}");
    Ok(())
}

macro_rules! pymodule {
    ($pyfile:literal) => {
        pub static PY_MODULE: once_cell::sync::Lazy<eyre::Result<pyo3::Py<pyo3::types::PyModule>>> =
            once_cell::sync::Lazy::new(|| {
                pyo3::Python::with_gil(|py| -> eyre::Result<pyo3::Py<pyo3::types::PyModule>> {
                    use crate::pydriver::venv::TracebackError;
                    let src = c_str!(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $pyfile)));
                    let module = pyo3::types::PyModule::from_code(
                        py,
                        src,
                        c_str!("module.py"),
                        c_str!("__main__"),
                    )
                    .format_traceback(py)?;
                    module.add_function(wrap_pyfunction!($crate::pydriver::venv::info, &module)?)?;
                    Ok(module.into())
                })
            });
    };
}

macro_rules! get_module {
    ($module:ident) => {
        match $module.as_ref() {
            Ok(module) => module,
            Err(e) => eyre::bail!(e),
        }
    };
}

pymodule!("/src/pydriver/venv/venv.py");

pub fn with_venv(py: Python<'_>) -> Result<bool> {
    const THIS_VENV: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/.venv");
    dbg!(THIS_VENV);
    let activate_venv = get_module!(PY_MODULE).getattr(py, "activate_venv")?;
    let result = activate_venv.call1(py, (PyString::new(py, THIS_VENV),))?;

    // Debug: Print sys.path after activation
    let get_sys_path = get_module!(PY_MODULE).getattr(py, "get_sys_path")?;
    let sys_path = get_sys_path.call0(py)?;
    println!("Python sys.path after activation: {:?}", sys_path);

    Ok(result.extract(py)?)
}
