use pgrx::{GucContext, GucFlags, GucRegistry, GucSetting};
use std::ffi::CStr;

pub static VENV: GucSetting<Option<&'static CStr>> = GucSetting::<Option<&'static CStr>>::new(None);

pub fn init() {
    GucRegistry::define_string_guc(
        "pgpt.venv",
        "Python's virtual environment path",
        "",
        &VENV,
        GucContext::Userset,
        GucFlags::default(),
    );
}
