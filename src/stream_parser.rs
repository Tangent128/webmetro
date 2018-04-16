use bytes::BytesMut;
use bytes::BufMut;
use futures::Async;
use futures::stream::Stream;

use ebml::EbmlEventSource;
use ebml::FromEbml;
use error::WebmetroError;

pub struct EbmlStreamingParser<S> {
    stream: S,
    buffer: BytesMut,
    buffer_size_limit: Option<usize>,
    last_read: usize
}

impl<S> EbmlStreamingParser<S> {
    pub fn with_buffer_limit(mut self, limit: usize) -> Self {
        self.buffer_size_limit = Some(limit);
        self
    }
}

pub trait StreamEbml where Self: Sized + Stream, Self::Item: AsRef<[u8]> {
    fn parse_ebml(self) -> EbmlStreamingParser<Self> {
        EbmlStreamingParser {
            stream: self,
            buffer: BytesMut::new(),
            buffer_size_limit: None,
            last_read: 0
        }
    }
}

impl<I: AsRef<[u8]>, S: Stream<Item = I, Error = WebmetroError>> StreamEbml for S {}

impl<I: AsRef<[u8]>, S: Stream<Item = I, Error = WebmetroError>> EbmlStreamingParser<S> {
    pub fn poll_event<'a, T: FromEbml<'a>>(&'a mut self) -> Result<Async<Option<T>>, WebmetroError> {
        // release buffer from previous event
        self.buffer.advance(self.last_read);
        self.last_read = 0;

        loop {
            match T::check_space(&self.buffer) {
                Ok(None) => {
                    // need to refill buffer, below
                },
                other => return other.map_err(WebmetroError::EbmlError).and_then(move |_| {
                    match T::decode_element(&self.buffer) {
                        Err(err) => Err(WebmetroError::EbmlError(err)),
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
                Ok(Async::Ready(Some(chunk))) => {
                    self.buffer.reserve(chunk.as_ref().len());
                    self.buffer.put_slice(chunk.as_ref());
                    // ok can retry decoding now
                },
                other => return other.map(|async| async.map(|_| None))
            }
        }
    }
}

impl<I: AsRef<[u8]>, S: Stream<Item = I, Error = WebmetroError>> EbmlEventSource for EbmlStreamingParser<S> {
    type Error = WebmetroError;

    fn poll_event<'a, T: FromEbml<'a>>(&'a mut self) -> Result<Async<Option<T>>, WebmetroError> {
        return EbmlStreamingParser::poll_event(self);
    }
}

#[cfg(test)]
mod tests {
    //#[test]
    
}
