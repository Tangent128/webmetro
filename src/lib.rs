
extern crate bytes;
extern crate futures;

pub mod chunk;
pub mod ebml;
mod iterator;
pub mod slice;

pub mod webm_stream;
pub mod timecode_fixer;
pub mod webm;

pub use ebml::{EbmlError, FromEbml};

#[cfg(test)]
mod tests {
    use futures::future::{ok, Future};

    pub const TEST_FILE: &'static [u8] = include_bytes!("data/test1.webm");

    #[test]
    fn hello_futures() {
        let my_future = ok::<String, ()>("Hello".into())
            .map(|hello| hello + ", Futures!");

        let string_result = my_future.wait().unwrap();

        assert_eq!(string_result, "Hello, Futures!");
    }
}
