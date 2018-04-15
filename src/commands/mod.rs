use std::error::Error;
use std::io::{
    Error as IoError,
    ErrorKind,
    stdin,
    Stdin
};

use futures::{
    prelude::*,
    stream::MapErr
};
use hyper::Error as HyperError;
use tokio_io::{
    io::AllowStdIo,
    codec::{
        BytesCodec,
        FramedRead
    }
};
use webmetro::error::WebmetroError;

pub mod dump;
pub mod filter;
pub mod relay;

/// An adapter that makes chunks of bytes from stdin available as a Stream;
/// is NOT actually async, and just uses blocking read. Don't use more than
/// one at once, who knows who gets which bytes.
pub fn stdin_stream() -> MapErr<FramedRead<AllowStdIo<Stdin>, BytesCodec>, fn(IoError) -> WebmetroError> {
    FramedRead::new(AllowStdIo::new(stdin()), BytesCodec::new())
    .map_err(WebmetroError::IoError)
}

pub fn to_hyper_error(err: WebmetroError) -> HyperError {
    match err {
        WebmetroError::IoError(io_err) => io_err.into(),
        err => IoError::new(ErrorKind::InvalidData, err.description()).into()
    }
}
