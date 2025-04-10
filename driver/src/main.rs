use natural_driver::generator::SqlGenerator;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;

fn main() -> eyre::Result<()> {
    let backend = LlamaBackend::init()?;
    let model_params = LlamaModelParams::default();
    let model =
        LlamaModel::load_from_file(&backend, "/home/mara/Workspace/mistral.gguf", &model_params)?;
    let ctx_params = LlamaContextParams::default().with_n_threads(4);
    let context = model.new_context(&backend, ctx_params)?;

    let mut generator = SqlGenerator::new(context).unwrap();

    let schema = "CREATE TABLE users (id INT PRIMARY KEY, name TEXT, email TEXT);\n CREATE TABLE orders (id SERIAL PRIMARY KEY, product TEXT NOT NULL);";
    let query = "Find all users who are named henry";

    match generator.generate(query, schema) {
        Ok(sql) => println!("Generated SQL: {}", sql),
        Err(e) => eprintln!("Error: {}", e),
    }

    dbg!(schema, query);

    Ok(())
}
