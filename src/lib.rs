
pub mod ebml;
pub mod error;

pub mod iterator;
pub mod stream_parser;

pub mod chunk;
pub mod fixers;
pub mod webm;

pub mod channel;

pub use crate::ebml::{EbmlError, FromEbml};

#[cfg(test)]
mod tests {
    pub const TEST_FILE: &'static [u8] = include_bytes!("data/test1.webm");
    pub const ENCODE_WEBM_TEST_FILE: &'static [u8] = include_bytes!("data/encode_webm_test.webm");
}
