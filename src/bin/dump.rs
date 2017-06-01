extern crate lab_ebml;

use std::env::args;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use lab_ebml::{Schema, Webm};
use lab_ebml::WebmElement::*;

pub fn main() {
    let mut args = args();
    let _ = args.next();
    let filename = args.next().expect("Reading filename");

    let mut buffer = Vec::new();
    let mut file = File::open(Path::new(&filename)).expect("Opening file");

    file.read_to_end(&mut buffer).expect("Reading file contents");

    for element in Webm.parse(buffer.as_slice()) {
        match element {
            // suppress printing byte arrays
            Tracks(slice) => println!("Tracks[{}]", slice.len()),
            Cluster(slice) => println!("Cluster[{}]", slice.len()),
            other => println!("{:?}", other)
        }
    }

}
