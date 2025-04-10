use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::ggml_time_us;
use sqlparser::ast::Statement;
use std::time::Duration;

use eyre::{ensure, eyre, Context, ContextCompat, Result};
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::{AddBos, Special};
use llama_cpp_2::sampling::LlamaSampler;
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;

const PROMPT: &str = r#"
You are an expert SQL query generator that converts natural language to SQL.

<schema>{SCHEMA}</schema>

<question>{QUESTION}</question>

Based on the schema, generate the most efficient SQL query that answers the question.
You can only reference tables and columns outlined in the schema!
Output SQL must be postgres compliant.

Output your response in this exact format:

<sql>
[OUTPUT SQL QUERY]
</sql>
"#;

pub struct SqlGenerator<'c> {
    context: llama_cpp_2::context::LlamaContext<'c>,
    dialect: PostgreSqlDialect,
}

impl<'c> SqlGenerator<'c> {
    pub fn new(context: LlamaContext<'c>) -> Result<Self> {
        Ok(Self {
            context,
            dialect: PostgreSqlDialect {},
        })
    }

    pub fn generate(&mut self, query: &str, schema: &str) -> Result<Statement> {
        let prompt = PROMPT
            .replace("{QUESTION}", query)
            .replace("{SCHEMA}", schema);

        let tokens = self.context.model.str_to_token(&prompt, AddBos::Always)?;

        let mut batch = LlamaBatch::new(512, 1);

        let last_index: i32 = (tokens.len() - 1) as i32;
        for (i, token) in (0_i32..).zip(tokens.into_iter()) {
            let is_last = i == last_index;
            batch.add(token, i, &[0], is_last)?;
        }

        self.context.decode(&mut batch)?;

        let mut output = String::new();

        let n_len = 1024;
        let mut n_cur = batch.n_tokens();

        let t_main_start = ggml_time_us();

        // The `Decoder`
        let mut decoder = encoding_rs::UTF_8.new_decoder();

        let mut sampler =
            LlamaSampler::chain_simple([LlamaSampler::dist(1234), LlamaSampler::greedy()]);

        while n_cur <= n_len {
            // sample the next token
            {
                let token = sampler.sample(&self.context, batch.n_tokens() - 1);

                sampler.accept(token);

                // is it an end of stream?
                if self.context.model.is_eog_token(token) {
                    eprintln!("DONE");
                    break;
                }

                let output_bytes = self
                    .context
                    .model
                    .token_to_bytes(token, Special::Tokenize)?;

                let mut decoded = String::with_capacity(64);

                let _decode_result = decoder.decode_to_string(&output_bytes, &mut decoded, false);

                dbg!(_decode_result);

                output.push_str(&decoded);

                batch.clear();
                batch.add(token, n_cur, &[0], true)?;
            }

            n_cur += 1;

            self.context
                .decode(&mut batch)
                .with_context(|| "failed to eval")?;
        }

        eprintln!("\n");

        let t_main_end = ggml_time_us();

        let duration = Duration::from_micros((t_main_end - t_main_start) as u64);
        dbg!(duration);

        let sql = dbg!(self.extract_sql(&output))?;

        let parsed = Parser::parse_sql(&self.dialect, &sql).context("Invalid SQL syntax")?;

        ensure!(
            parsed.len() == 1,
            "expected llm to output exactly one sql query"
        );

        Ok(parsed[0].clone())
    }

    fn extract_sql(&self, output: &str) -> eyre::Result<String> {
        output
            .lines()
            .filter(|line| !line.starts_with("```") && !line.trim().is_empty())
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .strip_prefix("<sql>")
            .wrap_err_with(|| {
                eyre!("Unexpected model output: Missing opening sql tag: {output:#?}")
            })?
            .strip_suffix("</sql>")
            .wrap_err_with(|| {
                eyre!("Unexpected model output: Missing closing sql tag: {output:#?}")
            })
            .map(|o| o.to_string())
    }
}
