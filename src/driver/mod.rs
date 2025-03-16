#![allow(unused)]

use std::sync::{self, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use eyre::eyre;
use generation::TextGenerator;
use uuid::Uuid;

use model::Model;

pub mod generation;
pub mod model;
pub mod utils;

static MODEL: LazyLock<model::Model> =
    LazyLock::new(|| Model::load("./resources").expect("loading the model failed"));

#[derive(Debug)]
struct Context {
    handle: Arc<Mutex<Option<DriverHandle>>>,
}

impl Context {
    fn try_replace(&self, handle: DriverHandle) -> eyre::Result<()> {
        let mut lock = self
            .handle
            .lock()
            .map_err(|_| eyre!("Failed to lock handle"))?;

        lock.replace(handle);

        Ok(())
    }
}

static CONTEXT: LazyLock<Context> = LazyLock::new(|| Context {
    handle: Arc::new(Mutex::new(None)),
});

pub struct Driver {
    model: &'static Model,
    prompts: Receiver<Prompt>,
    tokens: Sender<Token>,
}

impl Driver {
    /// Takes a while
    pub fn boot() -> Self {
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

        CONTEXT.try_replace(handle.clone()).unwrap();

        let driver = Self {
            // NOTE:
            // This ensures we are hitting the lazy lock
            // code and load the model into memory
            model: &MODEL,
            prompts: prx,
            tokens: ttx,
        };

        driver
    }

    pub fn attach() -> DriverHandle {
        loop {
            let Ok(handle) = CONTEXT.handle.try_lock() else {
                continue;
            };

            if handle.is_some() {
                break;
            }

            drop(handle);

            thread::sleep(Duration::from_millis(25));
        }

        DriverHandle::current()
    }

    /// Push the drivers event loop
    fn push(&mut self) {
        let prompt = self.prompts.recv().unwrap();

        let start = Instant::now();
        let model = self.model.clone();
        let end = Instant::now();

        println!(
            "seconds to clone model = {}",
            end.duration_since(start).as_secs()
        );

        let tokens = TextGenerator::new(model, 0, None, None, None, 1.1, 64)
            .run(prompt)
            .unwrap();

        for token in tokens {
            self.tokens.send(token).unwrap();
        }
    }
}

pub(self) struct Prompt {
    id: Uuid,
    payload: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(self) enum Token {
    Completion { prompt: Uuid, token: String },
    Eos { prompt: Uuid },
}

impl Token {
    /// Prompt ID this token belongs to
    pub fn pid(&self) -> Uuid {
        match self {
            Self::Eos { prompt } | Self::Completion { prompt, .. } => *prompt,
        }
    }
}

#[derive(Clone, Debug)]
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
            .lock()
            .unwrap()
            .clone()
            .expect("No pgpt driver thread is running")
    }

    /// Send a prompt to the driver
    pub fn prompt(&self, prompt: impl AsRef<str>) -> String {
        let id = Uuid::new_v4();

        self.prompt
            .send(Prompt {
                id,
                payload: prompt.as_ref().to_string(),
            })
            .expect("Driver must be running send prompts");

        let mut tokens = vec![];

        while let Ok(token) = self.token.recv() {
            if token.pid() != id {
                continue;
            }

            match token {
                Token::Completion { prompt, token } => tokens.push(token),
                Token::Eos { prompt } => break,
            }
        }

        tokens.join("")
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use uuid::Uuid;

    use crate::driver::{DriverHandle, Token, CONTEXT};

    use super::Driver;

    #[test]
    fn setup() {
        thread::spawn(|| {
            let mut driver = Driver::boot();

            loop {
                driver.push();
            }
        });

        let handle = Driver::attach();

        let answer = handle.prompt("Hi mistral!");

        assert_eq!(answer, "Hi!");
    }
}
