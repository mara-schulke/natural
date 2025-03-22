use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use eyre::{eyre, Context, Result};
use llama_cpp::LlamaModel;

#[derive(Clone)]
pub struct Model(pub(crate) LlamaModel);

impl Model {
    pub fn load<P>(model: &P) -> eyre::Result<Self>
    where
        P: AsRef<Path> + ?Sized,
    {
        let model = model.as_ref().to_path_buf();

        std::fs::remove_file("/tmp/mistral.gguf").ok();
        std::fs::soft_link(model, "/tmp/mistral.gguf").unwrap();

        let model =
            LlamaModel::load_from_file("/tmp/mistral.gguf", llama_cpp::LlamaParams::default())?;

        Ok(Self(model))
    }
}
