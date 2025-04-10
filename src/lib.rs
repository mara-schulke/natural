use std::time::Duration;

use pgrx::bgworkers::*;
use pgrx::prelude::*;

::pgrx::pg_module_magic!();

//extension_sql!(
//r#"
//CREATE TABLE history (
//id serial8 not null primary key,
//query text,
//output text
//);
//"#,
//name = "pgpt_conversations",
//);

#[pg_extern]
fn query(query: &str) -> String {
    use natural_driver::generator::SqlGenerator;

    use llama_cpp_2::context::params::LlamaContextParams;
    use llama_cpp_2::llama_backend::LlamaBackend;
    use llama_cpp_2::model::params::LlamaModelParams;
    use llama_cpp_2::model::LlamaModel;

    let backend = LlamaBackend::init().unwrap();
    let model_params = LlamaModelParams::default();
    let model =
        LlamaModel::load_from_file(&backend, "/home/mara/Workspace/mistral.gguf", &model_params)
            .unwrap();
    let ctx_params = LlamaContextParams::default().with_n_threads(4);
    let context = model.new_context(&backend, ctx_params).unwrap();

    let mut generator = SqlGenerator::new(context).unwrap();

    let schema = "CREATE TABLE users (id INT PRIMARY KEY, name TEXT, email TEXT);\n CREATE TABLE orders (id SERIAL PRIMARY KEY, product TEXT NOT NULL);";

    generator.generate(query, schema).unwrap().to_string()
}

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

#[pg_extern(immutable, parallel_safe)]
fn spi_query_random_id() -> Result<Option<i64>, pgrx::spi::Error> {
    Spi::get_one("SELECT id FROM spi.spi_example ORDER BY random() LIMIT 1")
}

#[pg_extern]
fn spi_query_title(title: &str) -> Result<Option<i64>, pgrx::spi::Error> {
    Spi::get_one_with_args(
        "SELECT id FROM spi.spi_example WHERE title = $1;",
        &[title.into()],
    )
}

#[pg_extern]
fn spi_query_by_id(id: i64) -> Result<Option<String>, spi::Error> {
    let (returned_id, title) = Spi::connect(|client| {
        let tuptable = client
            .select(
                "SELECT id, title FROM spi.spi_example WHERE id = $1",
                None,
                &[id.into()],
            )?
            .first();

        tuptable.get_two::<i64, String>()
    })?;

    info!("id={:?}", returned_id);
    Ok(title)
}

#[pg_extern]
fn spi_insert_title(title: &str) -> Result<Option<i64>, spi::Error> {
    Spi::get_one_with_args(
        "INSERT INTO spi.spi_example(title) VALUES ($1) RETURNING id",
        &[title.into()],
    )
}

#[pg_extern]
fn spi_insert_title2(
    title: &str,
) -> TableIterator<(name!(id, Option<i64>), name!(title, Option<String>))> {
    let tuple = Spi::get_two_with_args(
        "INSERT INTO spi.spi_example(title) VALUES ($1) RETURNING id, title",
        &[title.into()],
    )
    .unwrap();

    TableIterator::once(tuple)
}

#[pg_extern]
fn issue1209_fixed() -> Result<Option<String>, Box<dyn std::error::Error>> {
    let res = Spi::connect(|c| {
        let mut cursor = c.try_open_cursor("SELECT 'hello' FROM generate_series(1, 10000)", &[])?;
        let table = cursor.fetch(10000)?;
        table
            .into_iter()
            .map(|row| row.get::<&str>(1))
            .collect::<Result<Vec<_>, _>>()
    })?;

    Ok(res.first().cloned().flatten().map(|s| s.to_string()))
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

#[pg_guard]
#[no_mangle]
pub extern "C-unwind" fn natural_inference_worker(arg: pg_sys::Datum) {
    let arg = unsafe { i32::from_polymorphic_datum(arg, false, pg_sys::INT4OID) };

    // these are the signals we want to receive.  If we don't attach the SIGTERM handler, then
    // we'll never be able to exit via an external notification
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGHUP | SignalWakeFlags::SIGTERM);

    // we want to be able to use SPI against the specified database (postgres), as the superuser which
    // did the initdb. You can specify a specific user with Some("my_user")
    BackgroundWorker::connect_worker_to_spi(Some("postgres"), None);

    log!(
        "Hello from inside the {} BGWorker!  Argument value={}",
        BackgroundWorker::get_name(),
        arg.unwrap()
    );

    log!(
        "Goodbye from inside the {} BGWorker! ",
        BackgroundWorker::get_name()
    );
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_hello_pgpt() {}
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
