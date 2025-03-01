use candle_core::utils::{cuda_is_available, metal_is_available};
use candle_core::{Device, Tensor};
use eyre::{Context, Error, Result};

pub fn device(cpu: bool) -> Result<Device> {
    if cpu {
        Ok(Device::Cpu)
    } else if cuda_is_available() {
        Ok(Device::new_cuda(0)?)
    } else if metal_is_available() {
        Ok(Device::new_metal(0)?)
    } else {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            println!(
                "Running on CPU, to run on GPU(metal), build this example with `--features metal`"
            );
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            println!("Running on CPU, to run on GPU, build this example with `--features cuda`");
        }
        Ok(Device::Cpu)
    }
}

/// Loads the safetensors files for a model from the hub based on a json index file.
pub fn hub_load_safetensors(
    repo: &hf_hub::api::sync::ApiRepo,
    json_file: &str,
) -> Result<Vec<std::path::PathBuf>> {
    let json_file = repo.get(json_file).wrap_err("failed to get json file")?;

    let json_file = std::fs::File::open(json_file)?;
    let json: serde_json::Value =
        serde_json::from_reader(&json_file).wrap_err("failed to deser json file")?;

    let weight_map = match json.get("weight_map") {
        None => eyre::bail!("no weight map in {json_file:?}"),
        Some(serde_json::Value::Object(map)) => map,
        Some(_) => eyre::bail!("weight map in {json_file:?} is not a map"),
    };
    let mut safetensors_files = std::collections::HashSet::new();
    for value in weight_map.values() {
        if let Some(file) = value.as_str() {
            safetensors_files.insert(file.to_string());
        }
    }
    let safetensors_files = safetensors_files
        .iter()
        .map(|v| repo.get(v).wrap_err("failed to get repo"))
        .collect::<Result<Vec<_>>>()?;

    Ok(safetensors_files)
}

pub fn hub_load_local_safetensors<P: AsRef<std::path::Path>>(
    path: P,
    json_file: &str,
) -> Result<Vec<std::path::PathBuf>> {
    let path = path.as_ref();
    let jsfile = std::fs::File::open(path.join(json_file))?;
    let json: serde_json::Value =
        serde_json::from_reader(&jsfile).wrap_err("failed to read json file")?;

    let weight_map = match json.get("weight_map") {
        None => eyre::bail!("no weight map in {json_file:?}"),
        Some(serde_json::Value::Object(map)) => map,
        Some(_) => eyre::bail!("weight map in {json_file:?} is not a map"),
    };
    let mut safetensors_files = std::collections::HashSet::new();
    for value in weight_map.values() {
        if let Some(file) = value.as_str() {
            safetensors_files.insert(file);
        }
    }
    let safetensors_files: Vec<_> = safetensors_files
        .into_iter()
        .map(|v| path.join(v))
        .collect();
    Ok(safetensors_files)
}

pub mod token_output_stream {
    use eyre::Result;

    /// This is a wrapper around a tokenizer to ensure that tokens can be returned to the user in a
    /// streaming way rather than having to wait for the full decoding.
    pub struct TokenOutputStream {
        tokenizer: tokenizers::Tokenizer,
        tokens: Vec<u32>,
        prev_index: usize,
        current_index: usize,
    }

    impl TokenOutputStream {
        pub fn new(tokenizer: tokenizers::Tokenizer) -> Self {
            Self {
                tokenizer,
                tokens: Vec::new(),
                prev_index: 0,
                current_index: 0,
            }
        }

        pub fn into_inner(self) -> tokenizers::Tokenizer {
            self.tokenizer
        }

        fn decode(&self, tokens: &[u32]) -> Result<String> {
            match self.tokenizer.decode(tokens, true) {
                Ok(str) => Ok(str),
                Err(err) => eyre::bail!("cannot decode: {err}"),
            }
        }

        // https://github.com/huggingface/text-generation-inference/blob/5ba53d44a18983a4de32d122f4cb46f4a17d9ef6/server/text_generation_server/models/model.py#L68
        pub fn next_token(&mut self, token: u32) -> Result<Option<String>> {
            let prev_text = if self.tokens.is_empty() {
                String::new()
            } else {
                let tokens = &self.tokens[self.prev_index..self.current_index];
                self.decode(tokens)?
            };
            self.tokens.push(token);
            let text = self.decode(&self.tokens[self.prev_index..])?;
            if text.len() > prev_text.len() && text.chars().last().unwrap().is_alphanumeric() {
                let text = text.split_at(prev_text.len());
                self.prev_index = self.current_index;
                self.current_index = self.tokens.len();
                Ok(Some(text.1.to_string()))
            } else {
                Ok(None)
            }
        }

        pub fn decode_rest(&self) -> Result<Option<String>> {
            let prev_text = if self.tokens.is_empty() {
                String::new()
            } else {
                let tokens = &self.tokens[self.prev_index..self.current_index];
                self.decode(tokens)?
            };
            let text = self.decode(&self.tokens[self.prev_index..])?;
            if text.len() > prev_text.len() {
                let text = text.split_at(prev_text.len());
                Ok(Some(text.1.to_string()))
            } else {
                Ok(None)
            }
        }

        pub fn decode_all(&self) -> Result<String> {
            self.decode(&self.tokens)
        }

        pub fn get_token(&self, token_s: &str) -> Option<u32> {
            self.tokenizer.get_vocab(true).get(token_s).copied()
        }

        pub fn tokenizer(&self) -> &tokenizers::Tokenizer {
            &self.tokenizer
        }

        pub fn clear(&mut self) {
            self.tokens.clear();
            self.prev_index = 0;
            self.current_index = 0;
        }
    }
}
