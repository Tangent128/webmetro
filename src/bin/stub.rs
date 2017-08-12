extern crate lab_ebml;

use std::io::{Cursor, stdout, Write};
use lab_ebml::webm::*;

pub fn main() {
    let mut cursor = Cursor::new(Vec::new());

    encode_webm_element(WebmElement::EbmlHead, &mut cursor).unwrap();
    encode_webm_element(WebmElement::Segment, &mut cursor).unwrap();

    encode_webm_element(WebmElement::Cluster, &mut cursor).unwrap();
    encode_webm_element(WebmElement::Timecode(0), &mut cursor).unwrap();

    encode_webm_element(WebmElement::SimpleBlock {
        track: 3,
        flags: 0x0,
        timecode: 123,
        data: "Hello, World".as_bytes()
    }, &mut cursor).unwrap();

    encode_webm_element(WebmElement::Cluster, &mut cursor).unwrap();
    encode_webm_element(WebmElement::Timecode(1000), &mut cursor).unwrap();

    stdout().write_all(&cursor.get_ref()).unwrap();
}
