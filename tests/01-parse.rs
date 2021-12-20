#![allow(dead_code)]
use std::borrow::Cow;

use derive_into_owned::IntoOwned;

#[derive(IntoOwned)]
pub struct Parsed<'a> {
    content: Cow<'a, String>,
    string: String,
    number: u32,
}

// impl<'a> Parsed<'a> {
//     fn to_owned(self) -> Parsed<'static> {
//         Parsed {
//             content: Cow::Owned(self.content.into_owned()),
//             string: self.string,
//             number: self.number,
//         }
//     }
// }

fn main() {}
