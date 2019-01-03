use std::io::stdin;

use bytes::{
    Buf,
    IntoBuf
};
use futures::prelude::*;
use tokio_io::io::AllowStdIo;
use tokio_codec::{
    BytesCodec,
    FramedRead
};
use webmetro::error::WebmetroError;

pub mod dump;
pub mod filter;
pub mod relay;
pub mod send;

/// An adapter that makes chunks of bytes from stdin available as a Stream;
/// is NOT actually async, and just uses blocking read. Don't use more than
/// one at once, who knows who gets which bytes.
pub fn stdin_stream() -> impl Stream<Item = impl Buf, Error = WebmetroError> {
    FramedRead::new(AllowStdIo::new(stdin()), BytesCodec::new())
    .map(|bytes| bytes.into_buf())
    .map_err(WebmetroError::from)
}
