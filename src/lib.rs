
pub mod ebml;
pub mod error;
pub mod iterator;
pub mod slice;
pub mod stream_parser;

pub mod chunk;
pub mod fixers;
pub mod webm;

pub mod channel;

pub use crate::ebml::{EbmlError, FromEbml};

#[cfg(test)]
mod tests {
    use futures::future::{ok, Future};

    pub const TEST_FILE: &'static [u8] = include_bytes!("data/test1.webm");
    pub const ENCODE_WEBM_TEST_FILE: &'static [u8] = include_bytes!("data/encode_webm_test.webm");

    #[test]
    fn hello_futures() {
        let my_future = ok::<String, ()>("Hello".into())
            .map(|hello| hello + ", Futures!");

        let string_result = my_future.wait().unwrap();

        assert_eq!(string_result, "Hello, Futures!");
    }
}
