/// this ought to not compile, because
use std::borrow::Cow;

use derive_into_owned::IntoOwned;

// TODO: this already doesn't compile, but the test's still try to compile it anyway
#[derive(IntoOwned)]
pub struct Parsed<'a, T> {
    content: Cow<'a, String>,
    stuff: T,
}

fn main() {}
