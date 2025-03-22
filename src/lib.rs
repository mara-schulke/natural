use std::time::Duration;

use pgrx::bgworkers::*;
use pgrx::prelude::*;
use pyo3::types::PyString;

//use driver::Driver;
//use driver::Token;

::pgrx::pg_module_magic!();

pub mod driver;
pub mod pydriver;

extension_sql!(
    r#"
    CREATE TABLE history (
        id serial8 not null primary key,
        query text,
        output text
    );
    "#,
    name = "conversations",
);

#[pg_extern]
fn lol(prompt: &str) -> String {
    use crate::pydriver::venv;
    use pyo3::Python;
    use pyo3_ffi::c_str;

    Python::with_gil(|py| {
        dbg!(venv::with_venv(py).unwrap());
    });

    println!("activated");

    let code = c_str!(
        r#"
from enum import Enum
from pydantic import BaseModel, constr

import outlines
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer

class Weapon(str, Enum):
    sword = "sword"
    axe = "axe"
    mace = "mace"
    spear = "spear"
    bow = "bow"
    crossbow = "crossbow"


class Armor(str, Enum):
    leather = "leather"
    chainmail = "chainmail"
    plate = "plate"


class Character(BaseModel):
    name: constr(max_length=10)
    age: int
    armor: Armor
    weapon: Weapon
    strength: int

def complete():
    model_path = "/Users/mara.schulke/Documents/Private/ai/pgpt/resources"
    tokenizer = AutoTokenizer.from_pretrained(model_path)
    model = AutoModelForCausalLM.from_pretrained(
        model_path,
        torch_dtype=torch.float16,
        device_map="cpu"
    )

    generator = outlines.generate.json(model, Character)

    seed = 789001

    character = generator("Give me a character description", seed=seed)

    print(repr(character))

    character = generator("Give me an interesting character description")

    print(repr(character))
"#
    );

    Python::with_gil(|py| {
        use pyo3::prelude::PyModule;
        use pyo3::prelude::*;
        use pyo3::types::IntoPyDict;
        use pyo3::types::PyAnyMethods;
        use pyo3::PyAny;

        let numpy = PyModule::import(py, "numpy")?;
        dbg!(numpy);

        let torch = PyModule::import(py, "torch")?;
        dbg!(torch);

        let transformers = PyModule::import(py, "transformers")?;
        dbg!(transformers);

        let tensorflow = PyModule::import(py, "tensorflow")?;
        dbg!(tensorflow);

        let complete: Py<PyAny> = PyModule::from_code(py, code, c_str!(""), c_str!(""))?
            .getattr("complete")?
            .into();

        complete.call(py, (), None)?.extract::<Py<PyString>>(py)
    })
    .unwrap();

    "worked".to_string()
}

#[pg_extern]
fn query(query: &str) -> eyre::Result<String> {
    use pyo3::prelude::*;
    use pyo3_ffi::c_str;

    let py_code = c_str!(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test.py"
    )));

    Python::with_gil(|py| -> PyResult<Py<PyAny>> {
        crate::pydriver::venv::with_venv(py).unwrap();

        dbg!(std::env::var("PYTHONPATH"));

        match py.import("torch") {
            Ok(torch) => {
                println!("PyTorch imported successfully in test!");

                dbg!(torch.getattr("__version__"));
            }
            Err(e) => {
                println!("PyTorch import failed in test: {}", e);
                assert!(false, "PyTorch import failed");
            }
        }

        let result = match py.import("transformers") {
            Ok(transformers) => {
                println!("Transformers imported successfully");
                format!("Successfully processed: {transformers:#?}")
            }
            Err(e) => format!("Error importing transformers: {}", e),
        };

        dbg!(result);

        dbg!(py.version_info());

        let app: Py<PyAny> = PyModule::from_code(py, py_code, c_str!(""), c_str!(""))?
            .getattr("run")?
            .into();

        app.call0(py)
    })?;

    Ok(String::new())
}

#[pg_extern]
fn query_uds(query: &str) -> String {
    use serde::{Deserialize, Serialize};
    use serde_json;
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;

    let socket_path = "/tmp/pgptid";

    let mut stream = UnixStream::connect(socket_path).unwrap();

    serde_json::to_writer(&mut stream, &query).unwrap();

    let mut buffer = Vec::new();
    stream.read_to_end(&mut buffer).unwrap();

    let answer: String = serde_json::from_slice(&buffer).unwrap();

    println!("Received: {:?}", answer);

    answer
}

#[pg_guard]
pub extern "C-unwind" fn _PG_init() {
    pydriver::config::init();

    //dbg!(pydriver::venv::activate().expect("Error setting python venv"));

    BackgroundWorkerBuilder::new("PGPT Inference Worker")
        .set_function("pgpt_inference_worker")
        .set_library("pgpt")
        .set_argument(42i32.into_datum())
        .enable_spi_access()
        .load();
}

#[pg_guard]
#[no_mangle]
pub extern "C-unwind" fn pgpt_inference_worker(arg: pg_sys::Datum) {
    let arg = unsafe { i32::from_polymorphic_datum(arg, false, pg_sys::INT4OID) };

    // these are the signals we want to receive.  If we don't attach the SIGTERM handler, then
    // we'll never be able to exit via an external notification
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGHUP | SignalWakeFlags::SIGTERM);

    // we want to be able to use SPI against the specified database (postgres), as the superuser which
    // did the initdb. You can specify a specific user with Some("my_user")
    BackgroundWorker::connect_worker_to_spi(Some("postgres"), None);

    dbg!(std::env::var("PWD"));

    log!(
        "Hello from inside the {} BGWorker!  Argument value={}",
        BackgroundWorker::get_name(),
        arg.unwrap()
    );

    //let mut driver = Driver::boot();

    //while BackgroundWorker::wait_latch(Some(Duration::from_millis(25))) {
    //driver.push();
    //}

    log!(
        "Goodbye from inside the {} BGWorker! ",
        BackgroundWorker::get_name()
    );
}

//#[pg_extern]
//fn spi_return_query() -> Result<
//TableIterator<'static, (name!(oid, Option<pg_sys::Oid>), name!(name, Option<String>))>,
//spi::Error,
//> {
//#[cfg(feature = "pg12")]
//let query = "SELECT oid, relname::text || '-pg12' FROM pg_class";
//#[cfg(feature = "pg13")]
//let query = "SELECT oid, relname::text || '-pg13' FROM pg_class";
//#[cfg(feature = "pg14")]
//let query = "SELECT oid, relname::text || '-pg14' FROM pg_class";
//#[cfg(feature = "pg15")]
//let query = "SELECT oid, relname::text || '-pg15' FROM pg_class";
//#[cfg(feature = "pg16")]
//let query = "SELECT oid, relname::text || '-pg16' FROM pg_class";
//#[cfg(feature = "pg17")]
//let query = "SELECT oid, relname::text || '-pg17' FROM pg_class";

//Spi::connect(|client| {
//client
//.select(query, None, &[])?
//.map(|row| Ok((row["oid"].value()?, row[2].value()?)))
//.collect::<Result<Vec<_>, _>>()
//})
//.map(TableIterator::new)
//}

//#[pg_extern(immutable, parallel_safe)]
//fn spi_query_random_id() -> Result<Option<i64>, pgrx::spi::Error> {
//Spi::get_one("SELECT id FROM spi.spi_example ORDER BY random() LIMIT 1")
//}

//#[pg_extern]
//fn spi_query_title(title: &str) -> Result<Option<i64>, pgrx::spi::Error> {
//Spi::get_one_with_args(
//"SELECT id FROM spi.spi_example WHERE title = $1;",
//&[title.into()],
//)
//}

//#[pg_extern]
//fn spi_query_by_id(id: i64) -> Result<Option<String>, spi::Error> {
//let (returned_id, title) = Spi::connect(|client| {
//let tuptable = client
//.select(
//"SELECT id, title FROM spi.spi_example WHERE id = $1",
//None,
//&[id.into()],
//)?
//.first();

//tuptable.get_two::<i64, String>()
//})?;

//info!("id={:?}", returned_id);
//Ok(title)
//}

//#[pg_extern]
//fn spi_insert_title(title: &str) -> Result<Option<i64>, spi::Error> {
//Spi::get_one_with_args(
//"INSERT INTO spi.spi_example(title) VALUES ($1) RETURNING id",
//&[title.into()],
//)
//}

//#[pg_extern]
//fn spi_insert_title2(
//title: &str,
//) -> TableIterator<(name!(id, Option<i64>), name!(title, Option<String>))> {
//let tuple = Spi::get_two_with_args(
//"INSERT INTO spi.spi_example(title) VALUES ($1) RETURNING id, title",
//&[title.into()],
//)
//.unwrap();

//TableIterator::once(tuple)
//}

//#[pg_extern]
//fn issue1209_fixed() -> Result<Option<String>, Box<dyn std::error::Error>> {
//let res = Spi::connect(|c| {
//let mut cursor = c.try_open_cursor("SELECT 'hello' FROM generate_series(1, 10000)", &[])?;
//let table = cursor.fetch(10000)?;
//table
//.into_iter()
//.map(|row| row.get::<&str>(1))
//.collect::<Result<Vec<_>, _>>()
//})?;

//Ok(res.first().cloned().flatten().map(|s| s.to_string()))
//}

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
