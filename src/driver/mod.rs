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
//use generation::TextGenerator;
use uuid::Uuid;

use model::Model;

pub mod model;

static MODEL: LazyLock<model::Model> = LazyLock::new(|| {
    dbg!(concat!(env!("PWD"), "/mistral.gguf"));

    let start = Instant::now();

    let model =
        Model::load(concat!(env!("PWD"), "/mistral.gguf")).expect("loading the model failed");

    let end = Instant::now();

    println!(
        "seconds to load model = {}",
        end.duration_since(start).as_secs()
    );

    model
});

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

        Self {
            // NOTE:
            // This ensures we are hitting the lazy lock
            // code and load the model into memory
            model: &MODEL,
            prompts: prx,
            tokens: ttx,
        }
    }

    pub fn attach() -> DriverHandle {
        loop {
            dbg!("lock", &CONTEXT);
            let Ok(handle) = CONTEXT.handle.try_lock() else {
                continue;
            };

            dbg!("pre");
            if handle.is_some() {
                break;
            }

            drop(handle);

            dbg!("sleep");
            thread::sleep(Duration::from_millis(25));
        }

        DriverHandle::current()
    }

    /// Push the drivers event loop
    pub fn push(&mut self) {
        dbg!("recv");

        let prompt = self.prompts.recv().unwrap();

        dbg!("gen");

        let tokens = generation::Generator(&self.model).run(prompt).unwrap();

        dbg!("token");

        for token in tokens {
            self.tokens.send(token).unwrap();
        }
    }

    pub fn model(&self) -> &'static Model {
        self.model
    }
}

pub struct Prompt {
    id: Uuid,
    payload: String,
}

impl<T> From<T> for Prompt
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self {
            id: Uuid::new_v4(),
            payload: value.into(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Token {
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
pub struct DriverHandle {
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

pub mod generation {
    use llama_cpp::{standard_sampler::StandardSampler, SessionParams};

    use super::{model::Model, Prompt, Token};

    pub struct Generator(pub &'static Model);

    impl Generator {
        pub fn run(&self, prompt: Prompt) -> eyre::Result<Vec<Token>> {
            let mut ctx = self
                .0
                 .0
                .create_session(SessionParams::default())
                .expect("Failed to create session");

            ctx.set_context(prompt.payload).unwrap();

            let max_tokens = 128;
            let mut decoded_tokens = 0;

            let completions = ctx
                .start_completing_with(StandardSampler::default(), max_tokens)?
                .into_strings();

            let mut tokens = vec![];

            for completion in completions {
                dbg!(&completion);

                tokens.push(Token::Completion {
                    prompt: prompt.id,
                    token: completion,
                });

                decoded_tokens += 1;

                if decoded_tokens > max_tokens {
                    break;
                }
            }

            tokens.push(Token::Eos { prompt: prompt.id });

            Ok(tokens)
        }
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
