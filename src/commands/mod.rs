use std::io::{
    Cursor,
    Error as IoError,
    stdin,
    Stdin
};

use futures::{
    prelude::*,
    stream::MapErr
};
use hyper::body::Payload;
use tokio_io::io::AllowStdIo;
use tokio_codec::{
    BytesCodec,
    FramedRead
};
use webmetro::{
    chunk::Chunk,
    error::WebmetroError,
};

pub mod dump;
pub mod filter;
pub mod relay;
pub mod send;

/// An adapter that makes chunks of bytes from stdin available as a Stream;
/// is NOT actually async, and just uses blocking read. Don't use more than
/// one at once, who knows who gets which bytes.
pub fn stdin_stream() -> MapErr<FramedRead<AllowStdIo<Stdin>, BytesCodec>, fn(IoError) -> WebmetroError> {
    FramedRead::new(AllowStdIo::new(stdin()), BytesCodec::new())
    .map_err(WebmetroError::IoError)
}

/// A wrapper to make a Stream of Webm chunks work as a payload for Hyper
pub struct WebmPayload<S: Send + 'static>(pub S);

impl<S: Stream<Item = Chunk, Error = WebmetroError> + Send + 'static> Payload for WebmPayload<S> {
    type Data = Cursor<Chunk>;
    type Error = S::Error;

    fn poll_data(&mut self) -> Poll<Option<Cursor<Chunk>>, WebmetroError> {
        self.0.poll().map(|async| async.map(|option| option.map(Cursor::new)))
    }
}
