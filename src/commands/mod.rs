use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use tokio_util::codec::{BytesCodec, FramedRead};

pub mod dump;
pub mod filter;
pub mod relay;
pub mod send;

/// An adapter that makes chunks of bytes from stdin available as a Stream;
/// is NOT actually async, and just uses blocking read. Don't use more than
/// one at once, who knows who gets which bytes.
pub fn stdin_stream() -> impl Stream<Item = Result<Bytes, std::io::Error>> + Sized + Unpin {
    FramedRead::new(tokio::io::stdin(), BytesCodec::new())
        .map_ok(|bytes| bytes.freeze())
}
