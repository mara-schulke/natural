use std::path::{Path, PathBuf};

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
