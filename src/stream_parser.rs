use bytes::{
    Buf,
    BufMut,
    BytesMut
};
use futures::{
    Async,
    stream::Stream
};

use crate::ebml::{
    EbmlEventSource,
    FromEbml
};
use crate::error::WebmetroError;

pub struct EbmlStreamingParser<S> {
    stream: S,
    buffer: BytesMut,
    buffer_size_limit: Option<usize>,
    last_read: usize
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

pub trait StreamEbml where Self: Sized + Stream, Self::Item: Buf {
    fn parse_ebml(self) -> EbmlStreamingParser<Self> {
        EbmlStreamingParser {
            stream: self,
            buffer: BytesMut::new(),
            buffer_size_limit: None,
            last_read: 0
        }
    }
}

impl<I: Buf, S: Stream<Item = I, Error = WebmetroError>> StreamEbml for S {}

impl<I: Buf, S: Stream<Item = I, Error = WebmetroError>> EbmlStreamingParser<S> {
    pub fn poll_event<'a, T: FromEbml<'a>>(&'a mut self) -> Result<Async<Option<T>>, WebmetroError> {
        // release buffer from previous event
        self.buffer.advance(self.last_read);
        self.last_read = 0;

        loop {
            match T::check_space(&self.buffer) {
                Ok(None) => {
                    // need to refill buffer, below
                },
                other => return other.map_err(WebmetroError::from).and_then(move |_| {
                    match T::decode_element(&self.buffer) {
                        Err(err) => Err(err.into()),
                        Ok(None) => panic!("Buffer was supposed to have enough data to parse element, somehow did not."),
                        Ok(Some((element, element_size))) => {
                            self.last_read = element_size;
                            Ok(Async::Ready(Some(element)))
                        }
                    }
                })
            }

            if let Some(limit) = self.buffer_size_limit {
                if limit <= self.buffer.len() {
                    return Err(WebmetroError::ResourcesExceeded);
                }
            }

            match self.stream.poll() {
                Ok(Async::Ready(Some(buf))) => {
                    self.buffer.reserve(buf.remaining());
                    self.buffer.put(buf);
                    // ok can retry decoding now
                },
                other => return other.map(|async_status| async_status.map(|_| None))
            }
        }
    }
}

impl<I: Buf, S: Stream<Item = I, Error = WebmetroError>> EbmlEventSource for EbmlStreamingParser<S> {
    type Error = WebmetroError;

    fn poll_event<'a, T: FromEbml<'a>>(&'a mut self) -> Result<Async<Option<T>>, WebmetroError> {
        return EbmlStreamingParser::poll_event(self);
    }
}

#[cfg(test)]
mod tests {
    use bytes::IntoBuf;
    use futures::prelude::*;
    use futures::Async::*;
    use matches::assert_matches;

    use crate::stream_parser::*;
    use crate::tests::ENCODE_WEBM_TEST_FILE;
    use crate::webm::*;

    #[test]
    fn stream_webm_test() {
        let pieces = vec![
            &ENCODE_WEBM_TEST_FILE[0..20],
            &ENCODE_WEBM_TEST_FILE[20..40],
            &ENCODE_WEBM_TEST_FILE[40..],
        ];

        let mut stream_parser = futures::stream::iter_ok(pieces.iter())
            .map(|bytes| bytes.into_buf())
            .parse_ebml();

        assert_matches!(
            stream_parser.poll_event(),
            Ok(Ready(Some(WebmElement::EbmlHead)))
        );
        assert_matches!(
            stream_parser.poll_event(),
            Ok(Ready(Some(WebmElement::Segment)))
        );
        assert_matches!(
            stream_parser.poll_event(),
            Ok(Ready(Some(WebmElement::Tracks(_))))
        );
        assert_matches!(
            stream_parser.poll_event(),
            Ok(Ready(Some(WebmElement::Cluster)))
        );
        assert_matches!(
            stream_parser.poll_event(),
            Ok(Ready(Some(WebmElement::Timecode(0))))
        );
        assert_matches!(
            stream_parser.poll_event(),
            Ok(Ready(Some(WebmElement::SimpleBlock(_))))
        );
        assert_matches!(
            stream_parser.poll_event(),
            Ok(Ready(Some(WebmElement::Cluster)))
        );
        assert_matches!(
            stream_parser.poll_event(),
            Ok(Ready(Some(WebmElement::Timecode(1000))))
        );
    }
}
