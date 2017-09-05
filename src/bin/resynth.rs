extern crate lab_ebml;

use std::io::{Cursor, stdout, Write};
use lab_ebml::Schema;
use lab_ebml::webm::*;
use lab_ebml::webm::WebmElement::*;

const SRC_FILE: &'static [u8] = include_bytes!("../data/test1.webm");

pub fn main() {

    let mut head = Vec::new();
    let mut body = Vec::new();

    let mut reading_head = true;

    for element in Webm.parse(SRC_FILE) {
        match element {
            Cluster => reading_head = false,
            // TODO: skip elements not required for streaming
            Info => continue,
            Void => continue,
            Unknown(_) => continue,
            _ => (),
        }

        if reading_head {
            head.push(element);
        } else {
            body.push(element);
        }
    }

    let mut output = Vec::new();
    let mut cursor = Cursor::new(output);

    for element in head {
        encode_webm_element(element, &mut cursor).unwrap();
    }

    for element in body {
        encode_webm_element(element, &mut cursor).unwrap();
    }

    output = cursor.into_inner();
    stdout().write_all(&output).unwrap();
    output.clear();

}
