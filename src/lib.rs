use pgrx::bgworkers::*;
use pgrx::prelude::*;

::pgrx::pg_module_magic!();

// Legacy / alternative candle-based driver
// mod driver;

/// Driver invocation through an SQL function against a predefined SQL schema
///
/// Missing steps are:
/// 1. Dynamic schema loading & IR for the model
/// 2. Execution of generated SQL
#[pg_extern]
fn query(query: &str) -> String {
    use natural_driver::generator::SqlGenerator;

    use llama_cpp_2::context::params::LlamaContextParams;
    use llama_cpp_2::llama_backend::LlamaBackend;
    use llama_cpp_2::model::params::LlamaModelParams;
    use llama_cpp_2::model::LlamaModel;

    let backend = LlamaBackend::init().unwrap();
    let model_params = LlamaModelParams::default().with_n_gpu_layers(512);
    let model =
        LlamaModel::load_from_file(&backend, "/home/mara/Workspace/mistral.gguf", &model_params)
            .unwrap();
    let ctx_params = LlamaContextParams::default().with_n_threads(4);
    let context = model.new_context(&backend, ctx_params).unwrap();

    let mut generator = SqlGenerator::new(context).unwrap();

    let schema = "CREATE TABLE users (id INT PRIMARY KEY, name TEXT, email TEXT);\n CREATE TABLE orders (id SERIAL PRIMARY KEY, product TEXT NOT NULL);";

    generator.generate(query, schema).unwrap().to_string()
}

/// Example on how to use the server programming interface to query postgres
#[pg_extern]
fn spi_return_query() -> Result<
    TableIterator<'static, (name!(oid, Option<pg_sys::Oid>), name!(name, Option<String>))>,
    spi::Error,
> {
    #[cfg(feature = "pg12")]
    let query = "SELECT oid, relname::text || '-pg12' FROM pg_class";
    #[cfg(feature = "pg13")]
    let query = "SELECT oid, relname::text || '-pg13' FROM pg_class";
    #[cfg(feature = "pg14")]
    let query = "SELECT oid, relname::text || '-pg14' FROM pg_class";
    #[cfg(feature = "pg15")]
    let query = "SELECT oid, relname::text || '-pg15' FROM pg_class";
    #[cfg(feature = "pg16")]
    let query = "SELECT oid, relname::text || '-pg16' FROM pg_class";
    #[cfg(feature = "pg17")]
    let query = "SELECT oid, relname::text || '-pg17' FROM pg_class";

    Spi::connect(|client| {
        client
            .select(query, None, &[])?
            .map(|row| Ok((row["oid"].value()?, row[2].value()?)))
            .collect::<Result<Vec<_>, _>>()
    })
    .map(TableIterator::new)
}

#[pg_guard]
pub extern "C-unwind" fn _PG_init() {
    BackgroundWorkerBuilder::new("Natural Inference Worker")
        .set_function("natural_inference_worker")
        .set_library("natural")
        .set_argument(42i32.into_datum())
        .enable_spi_access()
        .load();
}

/// Example BG worker creation in PGRX for further usage
#[pg_guard]
#[no_mangle]
pub extern "C-unwind" fn natural_inference_worker(arg: pg_sys::Datum) {
    let arg = unsafe { i32::from_polymorphic_datum(arg, false, pg_sys::INT4OID) };

    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGHUP | SignalWakeFlags::SIGTERM);

    BackgroundWorker::connect_worker_to_spi(Some("postgres"), None);

    log!(
        "Hello from inside the {} natural inference worker!  Argument value={}",
        BackgroundWorker::get_name(),
        arg.unwrap()
    );

    log!(
        "Goodbye from inside of the {} natural inference worker! ",
        BackgroundWorker::get_name()
    );
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_hello_natural() {}
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
