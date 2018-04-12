use std::io::{
    Error as IoError,
    StdinLock,
    prelude::*
};

use futures::{
    Async,
    stream::Stream
};

pub mod dump;
pub mod filter;
pub mod relay;

/// A hackish adapter that makes chunks of bytes from stdin available as a Stream;
/// is NOT actually async, and just uses blocking read. Buffers aren't optimized either
/// and copy more than necessary.
pub struct StdinStream<'a> {
    buf_reader: StdinLock<'a>,
    read_bytes: usize
}

impl<'a> StdinStream<'a> {
    pub fn new(lock: StdinLock<'a>) -> Self {
        StdinStream {
            buf_reader: lock,
            read_bytes: 0
        }
    }
}

impl<'a> Stream for StdinStream<'a> {
    type Item = Vec<u8>;
    type Error = IoError;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        self.buf_reader.consume(self.read_bytes);
        let read_bytes = &mut self.read_bytes;
        self.buf_reader.fill_buf().map(|slice| {
            *read_bytes = slice.len();
            if *read_bytes > 0 {
                Async::Ready(Some(Into::<Vec<u8>>::into(slice)))
            } else {
                Async::Ready(None)
            }
        })
    }
}
