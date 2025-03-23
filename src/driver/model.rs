use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use candle_core::{DType, Device};
use candle_nn::VarBuilder;
use candle_transformers::models::mistral::{Config, Model as Mistral};
use eyre::{eyre, Context, Result};
use tokenizers::Tokenizer;

#[derive(Clone)]
pub struct Model {
    pub mistral: Mistral,
    pub device: Device,
    pub tokenizer: Tokenizer,
}

impl Model {
    pub fn load<P>(model: &P) -> eyre::Result<Self>
    where
        P: AsRef<Path> + ?Sized,
    {
        let model = model.as_ref();

        let start = std::time::Instant::now();

        let tokenizer_filename = model.join("tokenizer.json");

        let filenames = load_safetensors(model, "model.safetensors.index.json")?;

        println!("retrieved the files in {:?}", start.elapsed());

        let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(|e| eyre!(e))?;

        let start = std::time::Instant::now();

        let config = {
            let config_file = model.join("config.json");
            serde_json::from_slice(&std::fs::read(config_file)?)?
        };

        // Always try to use GPU for the time being
        let device = crate::driver::utils::device(false)?;

        let mistral = {
            let dtype = DType::BF16;
            let vb = unsafe { VarBuilder::from_mmaped_safetensors(&filenames, dtype, &device)? };

            Mistral::new(&config, vb)?
        };

        let duration = Instant::now().duration_since(start);

        dbg!(duration);

        Ok(Self {
            mistral,
            tokenizer,
            device,
        })
    }
}

fn load_safetensors(model: &Path, json_file: &str) -> Result<Vec<PathBuf>> {
    let json_file = model.join(json_file);

    let json_file = std::fs::File::open(json_file)?;
    let json: serde_json::Value =
        serde_json::from_reader(&json_file).wrap_err("failed to deser json file")?;

    let weight_map = match json.get("weight_map") {
        None => eyre::bail!("no weight map in {json_file:?}"),
        Some(serde_json::Value::Object(map)) => map,
        Some(_) => eyre::bail!("weight map in {json_file:?} is not a map"),
    };

    let mut safetensors_files = HashSet::new();

    for value in weight_map.values() {
        if let Some(file) = value.as_str() {
            safetensors_files.insert(file.to_string());
        }
    }

    let safetensors_files = safetensors_files
        .iter()
        .map(|v| model.join(v))
        .collect::<Vec<_>>();

    Ok(safetensors_files)
}
