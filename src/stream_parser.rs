use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::Async;
use futures3::stream::{Stream, StreamExt, TryStream};
use std::task::{Context, Poll};

use crate::ebml::FromEbml;
use crate::error::WebmetroError;

pub struct EbmlStreamingParser<S> {
    stream: S,
    buffer: BytesMut,
    buffer_size_limit: Option<usize>,
    borrowed: Bytes,
}

impl<S> EbmlStreamingParser<S> {
    /// add a "soft" buffer size limit; if the input buffer exceeds this size,
    /// error the stream instead of resuming. It's still possible for the buffer
    /// to exceed this size *after* a fill, so ensure input sizes are reasonable.
    pub fn with_soft_limit(mut self, limit: usize) -> Self {
        self.buffer_size_limit = Some(limit);
        self
    }
}

pub trait StreamEbml: Sized + TryStream + Unpin
where
    Self: Sized + TryStream + Unpin,
    Self::Ok: Buf,
{
    fn parse_ebml(self) -> EbmlStreamingParser<Self> {
        EbmlStreamingParser {
            stream: self,
            buffer: BytesMut::new(),
            buffer_size_limit: None,
            borrowed: Bytes::new(),
        }
    }
}

impl<I: Buf, S: Stream<Item = Result<I, WebmetroError>> + Unpin> StreamEbml for S {}

impl<I: Buf, S: Stream<Item = Result<I, WebmetroError>> + Unpin> EbmlStreamingParser<S> {
    pub fn poll_event<'a, T: FromEbml<'a>>(
        &'a mut self,
        cx: &mut Context,
    ) -> Result<Async<Option<T>>, WebmetroError> {
        loop {
            match T::check_space(&self.buffer)? {
                None => {
                    // need to refill buffer, below
                }
                Some(info) => {
                    let mut bytes = self.buffer.split_to(info.element_len).freeze();
                    bytes.advance(info.body_offset);
                    self.borrowed = bytes;
                    return Ok(Async::Ready(Some(T::decode(
                        info.element_id,
                        &self.borrowed,
                    )?)));
                }
            }

            if let Some(limit) = self.buffer_size_limit {
                if limit <= self.buffer.len() {
                    return Err(WebmetroError::ResourcesExceeded);
                }
            }

            match self.stream.poll_next_unpin(cx)? {
                Poll::Ready(Some(buf)) => {
                    self.buffer.reserve(buf.remaining());
                    self.buffer.put(buf);
                    // ok can retry decoding now
                }
                Poll::Ready(None) => return Ok(Async::Ready(None)),
                Poll::Pending => return Ok(Async::NotReady),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::IntoBuf;
    use futures::Async::*;
    use futures3::{
        future::poll_fn,
        stream::StreamExt,
        FutureExt,
    };
    use matches::assert_matches;

    use crate::stream_parser::*;
    use crate::tests::ENCODE_WEBM_TEST_FILE;
    use crate::webm::*;

    #[test]
    fn stream_webm_test() {
        poll_fn(|cx| {
            let pieces = vec![
                &ENCODE_WEBM_TEST_FILE[0..20],
                &ENCODE_WEBM_TEST_FILE[20..40],
                &ENCODE_WEBM_TEST_FILE[40..],
            ];

            let mut stream_parser = futures3::stream::iter(pieces.iter())
                .map(|bytes| Ok(bytes.into_buf()))
                .parse_ebml();

            assert_matches!(
                stream_parser.poll_event(cx),
                Ok(Ready(Some(WebmElement::EbmlHead)))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ok(Ready(Some(WebmElement::Segment)))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ok(Ready(Some(WebmElement::Tracks(_))))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ok(Ready(Some(WebmElement::Cluster)))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ok(Ready(Some(WebmElement::Timecode(0))))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ok(Ready(Some(WebmElement::SimpleBlock(_))))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ok(Ready(Some(WebmElement::Cluster)))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ok(Ready(Some(WebmElement::Timecode(1000))))
            );

            std::task::Poll::Ready(())
        })
        .now_or_never()
        .expect("Test succeeded without blocking");
    }
}
