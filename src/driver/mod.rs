#![allow(unused)]

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use uuid::Uuid;

use model::Model;

pub mod model;
pub mod utils;

static MODEL: LazyLock<model::Model> =
    LazyLock::new(|| Model::load("./resources").expect("loading the mode failed"));

struct Context {
    handle: RefCell<Option<DriverHandle>>,
}

const CONTEXT: Context = Context {
    handle: RefCell::new(None),
};

struct Driver {
    model: &'static Model,
    prompts: Receiver<Prompt>,
    tokens: Sender<Token>,
}

impl Driver {
    /// Detaches the driver from the current thread
    pub fn detach() -> DriverHandle {
        // NOTE:
        // Allow at most 2 concurrent prompts, as we are not running on
        // good hardware. This proves that we can do concurrent inference without allowing to
        // exhaust resources on small machines.
        //
        // If one runs this outside of a research context this limitation should be dropped.
        let (ptx, prx) = bounded::<Prompt>(2);
        let (ttx, trx) = unbounded::<Token>();

        let handle = DriverHandle {
            prompt: ptx,
            token: trx,
        };

        std::thread::spawn(|| {
            let mut driver = Self {
                // NOTE:
                // This ensures we are hitting the lazy lock
                // code and load the model into memory
                model: &MODEL,
                prompts: prx,
                tokens: ttx,
            };

            // Push the event loop
            loop {
                driver.push();
            }

            // Unset the handle to make it impossible to obtain new handles
            CONTEXT.handle.replace(None);
        });

        CONTEXT.handle.replace(Some(handle.clone()));

        handle
    }

    /// Push the drivers event loop
    fn push(&mut self) {
        todo!()
    }
}

struct Prompt {
    id: Uuid,
}

enum Token {
    Eof { prompt: Uuid },
}

#[derive(Clone)]
struct DriverHandle {
    prompt: Sender<Prompt>,
    token: Receiver<Token>,
}

impl DriverHandle {
    /// Obtain a handle to the current driver.
    ///
    /// If we are running in a context where there is no driver thread this function will panic.
    pub fn current() -> Self {
        CONTEXT
            .handle
            .borrow()
            .clone()
            .expect("No pgpt driver thread is running")
    }
}

#[cfg(test)]
mod tests {
    use super::Driver;

    #[test]
    fn setup() {
        let handle = Driver::detach();
    }
}
