use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::stream::{Stream, StreamExt, TryStream};
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
    ) -> Poll<Option<Result<T, WebmetroError>>> {
        loop {
            match T::check_space(&self.buffer)? {
                None => {
                    // need to refill buffer, below
                }
                Some(info) => {
                    let mut bytes = self.buffer.split_to(info.element_len).freeze();
                    bytes.advance(info.body_offset);
                    self.borrowed = bytes;
                    return Poll::Ready(Some(T::decode(
                        info.element_id,
                        &self.borrowed,
                    ).map_err(Into::into)));
                }
            }

            if let Some(limit) = self.buffer_size_limit {
                if limit <= self.buffer.len() {
                    return Poll::Ready(Some(Err(WebmetroError::ResourcesExceeded)));
                }
            }

            match self.stream.poll_next_unpin(cx)? {
                Poll::Ready(Some(buf)) => {
                    self.buffer.reserve(buf.remaining());
                    self.buffer.put(buf);
                    // ok can retry decoding now
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl<I: Buf, S: Stream<Item = Result<I, WebmetroError>> + Unpin> EbmlStreamingParser<S> {
    pub async fn next<'a, T: FromEbml<'a>>(&'a mut self) -> Result<Option<T>, WebmetroError> {
        loop {
            if let Some(info) = T::check_space(&self.buffer)? {
                let mut bytes = self.buffer.split_to(info.element_len).freeze();
                bytes.advance(info.body_offset);
                self.borrowed = bytes;
                return Ok(Some(T::decode(info.element_id, &self.borrowed)?));
            }

            if let Some(limit) = self.buffer_size_limit {
                if limit <= self.buffer.len() {
                    // hit our buffer limit and still nothing parsed
                    return Err(WebmetroError::ResourcesExceeded);
                }
            }

            match self.stream.next().await.transpose()? {
                Some(refill) => {
                    self.buffer.reserve(refill.remaining());
                    self.buffer.put(refill);
                }
                None => {
                    // Nothing left, we're done
                    return Ok(None);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::{future::poll_fn, stream::StreamExt, FutureExt};
    use matches::assert_matches;
    use std::task::Poll::*;

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

            let mut stream_parser = futures::stream::iter(pieces.iter())
                .map(|bytes| Ok(&bytes[..]))
                .parse_ebml();

            assert_matches!(
                stream_parser.poll_event(cx),
                Ready(Some(Ok(WebmElement::EbmlHead)))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ready(Some(Ok(WebmElement::Segment)))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ready(Some(Ok(WebmElement::Tracks(_))))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ready(Some(Ok(WebmElement::Cluster)))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ready(Some(Ok(WebmElement::Timecode(0))))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ready(Some(Ok(WebmElement::SimpleBlock(_))))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ready(Some(Ok(WebmElement::Cluster)))
            );
            assert_matches!(
                stream_parser.poll_event(cx),
                Ready(Some(Ok(WebmElement::Timecode(1000))))
            );

            std::task::Poll::Ready(())
        })
        .now_or_never()
        .expect("Test tried to block on I/O");
    }

    #[test]
    fn async_webm_test() {
        let pieces = vec![
            &ENCODE_WEBM_TEST_FILE[0..20],
            &ENCODE_WEBM_TEST_FILE[20..40],
            &ENCODE_WEBM_TEST_FILE[40..],
        ];

        async {
            let mut parser = futures::stream::iter(pieces.iter())
                .map(|bytes| Ok(&bytes[..]))
                .parse_ebml();

            assert_matches!(parser.next().await?, Some(WebmElement::EbmlHead));
            assert_matches!(parser.next().await?, Some(WebmElement::Segment));
            assert_matches!(parser.next().await?, Some(WebmElement::Tracks(_)));
            assert_matches!(parser.next().await?, Some(WebmElement::Cluster));
            assert_matches!(parser.next().await?, Some(WebmElement::Timecode(0)));
            assert_matches!(parser.next().await?, Some(WebmElement::SimpleBlock(_)));
            assert_matches!(parser.next().await?, Some(WebmElement::Cluster));
            assert_matches!(parser.next().await?, Some(WebmElement::Timecode(1000)));

            Result::<(), WebmetroError>::Ok(())
        }
        .now_or_never()
        .expect("Test tried to block on I/O")
        .expect("Parse failed");
    }
}
