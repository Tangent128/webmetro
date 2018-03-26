extern crate lab_ebml;

use std::env::args;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use lab_ebml::webm::{ parse_webm, SimpleBlock };
use lab_ebml::webm::WebmElement::*;

pub fn main() {
    let mut args = args();
    let _ = args.next();
    let filename = args.next().expect("Reading filename");

    let mut buffer = Vec::new();
    let mut file = File::open(Path::new(&filename)).expect("Opening file");

    file.read_to_end(&mut buffer).expect("Reading file contents");

    for element in parse_webm(buffer.as_slice()) {
        match element {
            // suppress printing byte arrays
            Tracks(slice) => println!("Tracks[{}]", slice.len()),
            SimpleBlock(SimpleBlock {timecode, ..}) => println!("SimpleBlock@{}", timecode),
            other => println!("{:?}", other)
        }
    }

}
