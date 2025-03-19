pub mod config;
pub mod venv;

#[cfg(feature = "python")]
#[macro_export]
macro_rules! pymodule {
    ($pyfile:literal) => {
        pub static PY_MODULE: once_cell::sync::Lazy<
            anyhow::Result<pyo3::Py<pyo3::types::PyModule>>,
        > = once_cell::sync::Lazy::new(|| {
            pyo3::Python::with_gil(|py| -> anyhow::Result<pyo3::Py<pyo3::types::PyModule>> {
                use $crate::bindings::TracebackError;
                let src = c_str!(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $pyfile)));

                let module = pyo3::types::PyModule::from_code(
                    py,
                    src,
                    c_str!("module.py"),
                    c_str!("__main__"),
                )
                .format_traceback(py)?;

                Ok(module.into())
            })
        });
    };
}

#[cfg(feature = "python")]
#[macro_export]
macro_rules! get_module {
    ($module:ident) => {
        match $module.as_ref() {
            Ok(module) => module,
            Err(e) => anyhow::bail!(e),
        }
    };
}
