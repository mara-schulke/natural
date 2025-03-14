#![allow(unused)]

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use model::Model;

pub mod model;
pub mod utils;

static MODEL: LazyLock<model::Model> =
    LazyLock::new(|| Model::load("./resources").expect("loading the mode failed"));

//struct Prompt<'m>(&'m Model);

struct Context {
    handle: RefCell<Option<DriverHandle>>,
}

const CONTEXT: Context = Context {
    handle: RefCell::new(None),
};

struct Driver {
    model: &'static Model,
}

impl Driver {
    fn run(self) {
        todo!()
    }

    /// Detaches the driver from the current thread
    pub fn detach() -> DriverHandle {
        std::thread::spawn(|| {
            // HINT:
            // this ensures we are hitting the lazy lock
            // code and load the model into memory
            let driver = Self { model: &MODEL };

            // Push the event loop
            driver.run()
        });

        DriverHandle {}
    }
}

struct DriverHandle {}

impl DriverHandle {
    /// This obtains a handle to the current driver.
    ///
    /// If we are running in a context where there is no driver thread this function will panic.
    pub fn current() -> Self {
        todo!();
    }
}
