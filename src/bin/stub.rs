extern crate lab_ebml;

use std::io::{Cursor, stdout, Write};
use lab_ebml::webm::*;

pub fn main() {
    let mut cursor = Cursor::new(Vec::new());

    encode_webm_element(WebmElement::EbmlHead, &mut cursor).unwrap();

    stdout().write_all(&cursor.get_ref()).unwrap();
}
