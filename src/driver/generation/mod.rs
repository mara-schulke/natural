use clap::Parser;
use eyre::{Error as E, Result};

use candle_core::{DType, Device, Tensor};
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::mistral::Model as Mistral;
use tokenizers::Tokenizer;

use crate::driver::model::Model;
use crate::driver::utils::token_output_stream::TokenOutputStream;

use super::{Prompt, Token};

pub struct TextGenerator {
    model: Model,
    tokenizer: TokenOutputStream,
    logits_processor: LogitsProcessor,
    repeat_penalty: f32,
    repeat_last_n: usize,
}

impl TextGenerator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model: Model,
        seed: u64,
        temp: Option<f64>,
        top_p: Option<f64>,
        top_k: Option<usize>,
        repeat_penalty: f32,
        repeat_last_n: usize,
    ) -> Self {
        let logits_processor = {
            let temperature = temp.unwrap_or(0.);
            let sampling = if temperature <= 0. {
                Sampling::ArgMax
            } else {
                match (top_k, top_p) {
                    (None, None) => Sampling::All { temperature },
                    (Some(k), None) => Sampling::TopK { k, temperature },
                    (None, Some(p)) => Sampling::TopP { p, temperature },
                    (Some(k), Some(p)) => Sampling::TopKThenTopP { k, p, temperature },
                }
            };
            LogitsProcessor::from_sampling(seed, sampling)
        };

        Self {
            tokenizer: TokenOutputStream::new(model.tokenizer.clone()),
            model,
            logits_processor,
            repeat_penalty,
            repeat_last_n,
        }
    }

    pub fn run(&mut self, prompt: Prompt) -> Result<Vec<Token>> {
        use std::io::Write;

        self.tokenizer.clear();

        let mut tokens = self
            .tokenizer
            .tokenizer()
            .encode(prompt.payload, false)
            .map_err(E::msg)?
            .get_ids()
            .to_vec();

        for &t in tokens.iter() {
            if let Some(t) = self.tokenizer.next_token(t)? {
                // skip user provided tokens.
                println!("skipping {t}")
            }
        }

        std::io::stdout().flush()?;

        let mut generated_tokens = 0usize;
        let eos_token = match self.tokenizer.get_token("</s>") {
            Some(token) => token,
            None => eyre::bail!("cannot find the </s> token"),
        };
        let start_gen = std::time::Instant::now();

        let mut generated = vec![];

        // At most generate 100 tokens
        for index in 0..100 {
            let context_size = if index > 0 { 1 } else { tokens.len() };
            let start_pos = tokens.len().saturating_sub(context_size);
            let ctxt = &tokens[start_pos..];
            let input = Tensor::new(ctxt, &self.model.device)?.unsqueeze(0)?;

            let logits = self.model.mistral.forward(&input, start_pos)?;
            let logits = logits.squeeze(0)?.squeeze(0)?.to_dtype(DType::BF16)?;
            let logits = if self.repeat_penalty == 1. {
                logits
            } else {
                let start_at = tokens.len().saturating_sub(self.repeat_last_n);

                candle_transformers::utils::apply_repeat_penalty(
                    &logits,
                    self.repeat_penalty,
                    &tokens[start_at..],
                )?
            };

            let next_token = self.logits_processor.sample(&logits)?;

            tokens.push(next_token);
            generated_tokens += 1;

            if next_token == eos_token {
                break;
            }

            if let Some(token) = self.tokenizer.next_token(next_token)? {
                generated.push(Token::Completion {
                    prompt: prompt.id,
                    token,
                });
            }
        }

        let dt = start_gen.elapsed();

        if let Some(rest) = self.tokenizer.decode_rest().map_err(E::msg)? {
            generated.push(Token::Completion {
                prompt: prompt.id,
                token: rest,
            });
        }

        generated.push(Token::Eos { prompt: prompt.id });

        Ok(generated)
    }
}
