use std::time::Duration;

use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use tokio_util::codec::{BytesCodec, FramedRead};
use webmetro::error::WebmetroError;

pub mod dump;
pub mod filter;
pub mod relay;
pub mod send;

/// An adapter that makes chunks of bytes from stdin available as a Stream;
/// is NOT actually async, and just uses blocking read. Don't use more than
/// one at once, who knows who gets which bytes.
pub fn stdin_stream() -> impl Stream<Item = Result<Bytes, std::io::Error>> + Sized + Unpin {
    FramedRead::new(tokio::io::stdin(), BytesCodec::new()).map_ok(|bytes| bytes.freeze())
}

pub fn parse_time(arg: Option<&str>) -> Result<Option<Duration>, WebmetroError> {
    match arg {
        Some(string) => match string.parse() {
            Ok(secs) => Ok(Some(Duration::from_secs(secs))),
            Err(err) => Err(WebmetroError::ApplicationError {
                message: err.to_string(),
            }),
        },
        None => Ok(None),
    }
}
