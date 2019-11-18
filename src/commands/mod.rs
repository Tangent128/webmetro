use std::io::Cursor;

use bytes::Bytes;
use futures3::TryStreamExt;
use webmetro::error::WebmetroError;

pub mod dump;
pub mod filter;
pub mod relay;
pub mod send;

/// An adapter that makes chunks of bytes from stdin available as a Stream;
/// is NOT actually async, and just uses blocking read. Don't use more than
/// one at once, who knows who gets which bytes.
pub fn stdin_stream() -> impl futures3::TryStream<
    Item = Result<Cursor<Bytes>, WebmetroError>,
    Ok = Cursor<Bytes>,
    Error = WebmetroError,
> + Sized
       + Unpin {
    tokio2::codec::FramedRead::new(tokio2::io::stdin(), tokio2::codec::BytesCodec::new())
        .map_ok(|bytes| Cursor::new(bytes.freeze()))
        .map_err(WebmetroError::from)
}
