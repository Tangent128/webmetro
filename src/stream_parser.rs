use bytes::BytesMut;
use bytes::BufMut;
use futures::Async;
use futures::stream::Stream;

use ebml::EbmlError;
use ebml::EbmlEventSource;
use ebml::FromEbml;

pub enum ParsingError<E> {
    EbmlError(EbmlError),
    OtherError(E)
}

pub struct EbmlStreamingParser<S> {
    stream: S,
    buffer: BytesMut,
    last_read: usize
}

pub trait StreamEbml<I: AsRef<[u8]>, S: Stream<Item = I>> {
    fn parse_ebml(self) -> EbmlStreamingParser<S>;
}

impl<I: AsRef<[u8]>, S: Stream<Item = I>> StreamEbml<I, S> for S {
    fn parse_ebml(self) -> EbmlStreamingParser<S> {
        EbmlStreamingParser {
            stream: self,
            buffer: BytesMut::new(),
            last_read: 0
        }
    }
}

impl<I: AsRef<[u8]>, S: Stream<Item = I>> EbmlStreamingParser<S> {
    pub fn poll_event<'a, T: FromEbml<'a>>(&'a mut self) -> Result<Async<Option<T>>, ParsingError<S::Error>> {
        // release buffer from previous event
        self.buffer.advance(self.last_read);
        self.last_read = 0;

        loop {
            match T::check_space(&self.buffer) {
                Err(err) => {
                    return Err(ParsingError::EbmlError(err))
                },
                Ok(None) => {
                    // need to refill buffer, below
                },
                Ok(Some(_)) => {
                    return match T::decode_element(&self.buffer) {
                        Err(err) => {
                            Err(ParsingError::EbmlError(err))
                        },
                        Ok(None) => {
                            // buffer should have the data already
                            panic!("Buffer was supposed to have enough data to parse element, somehow did not.")
                        },
                        Ok(Some((element, element_size))) => {
                            self.last_read = element_size;
                            return Ok(Async::Ready(Some(element)))
                        }
                    }
                }
            }

            match self.stream.poll() {
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Ok(Async::Ready(None)) => return Ok(Async::Ready(None)),
                Ok(Async::Ready(Some(chunk))) => {
                    self.buffer.reserve(chunk.as_ref().len());
                    self.buffer.put_slice(chunk.as_ref());
                    //println!("Read {} into Buffer", chunk.as_ref().len());
                    // ok can retry decoding now
                }
                Err(err) => return Err(ParsingError::OtherError(err))
            };
        }
    }
}

impl<I: AsRef<[u8]>, S: Stream<Item = I>> EbmlEventSource for EbmlStreamingParser<S> {
    type Error = ParsingError<S::Error>;

    fn poll_event<'a, T: FromEbml<'a>>(&'a mut self) -> Result<Async<Option<T>>, Self::Error> {
        return EbmlStreamingParser::poll_event(self);
    }
}

#[cfg(test)]
mod tests {
    //#[test]
    
}
