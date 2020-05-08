use std::io::Cursor;

use bytes::Bytes;
use futures::{TryStream, TryStreamExt};
use tokio_util::codec::{BytesCodec, FramedRead};
use webmetro::error::WebmetroError;

pub mod dump;
pub mod filter;
pub mod relay;
pub mod send;

/// An adapter that makes chunks of bytes from stdin available as a Stream;
/// is NOT actually async, and just uses blocking read. Don't use more than
/// one at once, who knows who gets which bytes.
pub fn stdin_stream() -> impl TryStream<
    Item = Result<Cursor<Bytes>, WebmetroError>,
    Ok = Cursor<Bytes>,
    Error = WebmetroError,
> + Sized
       + Unpin {
    FramedRead::new(tokio::io::stdin(), BytesCodec::new())
        .map_ok(|bytes| Cursor::new(bytes.freeze()))
        .map_err(WebmetroError::from)
}
