extern crate lab_ebml;

use std::io::{Cursor, stdout, Write};
use lab_ebml::webm::*;
use lab_ebml::webm::WebmElement::*;

const SRC_FILE: &'static [u8] = include_bytes!("../data/test1.webm");

pub fn main() {

    let mut head = Vec::new();
    let mut body = Vec::new();

    let mut reading_head = true;

    for element in parse_webm(SRC_FILE) {
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

    let mut cursor = Cursor::new(Vec::new());

    let mut fixer = TimecodeFixer::new();

    for element in &head {
        encode_webm_element(fixer.process(element), &mut cursor).unwrap();
    }

    for element in &body {
        encode_webm_element(fixer.process(element), &mut cursor).unwrap();
    }

    for element in &body {
        encode_webm_element(fixer.process(element), &mut cursor).unwrap();
    }

    let mut output = cursor.into_inner();
    stdout().write_all(&output).unwrap();
    output.clear();

}

pub struct TimecodeFixer {
    pub current_offset: u64,
    pub last_cluster_base: u64,
    pub last_observed_timecode: u64,
    pub assumed_duration: u64
}

impl TimecodeFixer {
    pub fn new() -> TimecodeFixer {
        TimecodeFixer {
            current_offset: 0,
            last_cluster_base: 0,
            last_observed_timecode: 0,
            assumed_duration: 33
        }
    }

    pub fn process<'b>(&mut self, element: &WebmElement<'b>) -> WebmElement<'b> {
        match element {
            &WebmElement::Timecode(timecode) => {
                // detect a jump backwards in the source, meaning we need to recalculate our offset
                if timecode < self.last_cluster_base {
                    let next_timecode = self.last_observed_timecode + self.assumed_duration;
                    self.current_offset = next_timecode - timecode;
                }

                // remember the source timecode to detect future jumps
                self.last_cluster_base = timecode;

                // return adjusted timecode
                WebmElement::Timecode(timecode + self.current_offset)
            },
            &WebmElement::SimpleBlock(block) => {
                self.last_observed_timecode = self.last_cluster_base + (block.timecode as u64);
                *element
            },
            _ => *element
        }
    }
}
