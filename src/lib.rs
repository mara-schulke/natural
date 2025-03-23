use pgrx::prelude::*;

use crate::driver::Prompt;
use crate::driver::Token;
use driver::Driver;

::pgrx::pg_module_magic!();

pub mod driver;
use crate::driver::generation::TextGenerator;

#[pg_guard]
pub extern "C-unwind" fn _PG_init() {
    let driver = Driver::boot();

    let tokens = TextGenerator::new(driver.model().clone(), 0, None, None, None, 1.1, 64)
        .run(dbg!(Prompt {
            id: uuid::Uuid::new_v4(),
            payload: "Hi model!".to_string(),
        }))
        .unwrap();

    dbg!("token");

    dbg!(&tokens);

    let mut result = vec![];

    for token in tokens {
        dbg!(&token);

        match token {
            Token::Completion { token, .. } => result.push(token),
            Token::Eos { .. } => break,
        }
    }

    dbg!(result.join(""));
}
