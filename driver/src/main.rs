//use llama_cpp::standard_sampler::StandardSampler;
//use llama_cpp::{LlamaModel, LlamaParams, SessionParams};
use std::io::{self, Write};

use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::ggml_time_us;
use std::time::Duration;

const MODEL_SETUP: &str = r#"
*This is a system message for context*

You MUST generate ONLY one VALID PostgreSQL query that queries the following SQL schema.

Here is the DB Schema:

CREATE TABLE customer (
 id INT AUTO_INCREMENT PRIMARY KEY,
 name VARCHAR(50) NOT NULL,
 postalCode VARCHAR(15) default NULL
);

CREATE TABLE product (
 id INT AUTO_INCREMENT PRIMARY KEY,
 product_name VARCHAR(50) NOT NULL,
 price VARCHAR(7) NOT NULL,
 qty VARCHAR(4) NOT NULL
);

CREATE TABLE order (
 id INT AUTO_INCREMENT PRIMARY KEY,
 product_id INT REFERENCES product(id) NOT NULL,
 customer_id INT REFERENCES customer(id) NOT NULL,
 at TIMESTAMPTZ NOT NULL
);


If you do string comparisons do them exact.
If you do time comparisons use postgres syntax.

You are expected to output NOTHING EXCEPT the SQL Query,
not a single character that is not part of the SQL query.
All outputs must comply to postgres syntax.

*END OF SYSTEM INSTRUCTIONS NOW THE ACTUAL PROMPT IS PROVIDED*

"#;

const MODEL_PROMPT: &str = r#"
Give me all orders made by "Herbert" today of that contain the product "Toothbrush".
"#;

use eyre::{Context, Result};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel, Special};
use llama_cpp_2::sampling::LlamaSampler;
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;

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

    pub fn generate(&mut self, query: &str, schema: &str) -> Result<String> {
        let prompt = format!(
            "Generate SQL for this request. Schema: {}\nRequest: {}\nOnly output the SQL query with no explanation:\n",
            schema, query
        );

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
                    eprintln!();
                    break;
                }

                let output_bytes = self
                    .context
                    .model
                    .token_to_bytes(token, Special::Tokenize)?;
                // use `Decoder.decode_to_string()` to avoid the intermediate buffer
                let mut decoded = String::with_capacity(32);

                let _decode_result = decoder.decode_to_string(&output_bytes, &mut decoded, false);

                output.push_str(&decoded);

                print!("{decoded}");
                std::io::stdout().flush()?;

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

        let sql = dbg!(self.extract_sql(&output));

        self.validate_sql(&sql)?;

        Ok(sql)
    }

    fn extract_sql(&self, output: &str) -> String {
        output
            .lines()
            .filter(|line| !line.starts_with("```") && !line.trim().is_empty())
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .to_string()
    }

    fn validate_sql(&self, sql: &str) -> Result<()> {
        Parser::parse_sql(&self.dialect, sql)
            .map(|_| ())
            .context("Invalid SQL syntax")
    }
}

fn main() -> eyre::Result<()> {
    let backend = LlamaBackend::init()?;
    let model_params = LlamaModelParams::default();
    let model = LlamaModel::load_from_file(&backend, "/tmp/mistral.gguf", &model_params)?;
    let ctx_params = LlamaContextParams::default().with_n_threads(4);
    let context = model.new_context(&backend, ctx_params)?;

    let mut generator = SqlGenerator::new(context).unwrap();

    let schema = "CREATE TABLE users (id INT PRIMARY KEY, name TEXT, email TEXT);";
    let query = "Find all users with gmail addresses";

    match generator.generate(query, schema) {
        Ok(sql) => println!("Generated SQL: {}", sql),
        Err(e) => eprintln!("Error: {}", e),
    }

    Ok(())
}
