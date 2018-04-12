use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult}
};

use bytes::BytesMut;
use bytes::BufMut;
use futures::Async;
use futures::stream::Stream;

use ebml::EbmlError;
use ebml::EbmlEventSource;
use ebml::FromEbml;

#[derive(Debug)]
pub enum ParsingError<E> {
    EbmlError(EbmlError),
    OtherError(E)
}
impl<E: Display + Error> Display for ParsingError<E> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "Parsing error: {}", self.description())
    }
}
impl<E: Error> Error for ParsingError<E> {
    fn description(&self) -> &str {
        match self {
            &ParsingError::EbmlError(ref err) => err.description(),
            &ParsingError::OtherError(ref err) => err.description()
        }
    }
}

pub struct EbmlStreamingParser<S> {
    stream: S,
    buffer: BytesMut,
    last_read: usize
}

pub trait StreamEbml where Self: Sized + Stream, Self::Item: AsRef<[u8]> {
    fn parse_ebml(self) -> EbmlStreamingParser<Self> {
        EbmlStreamingParser {
            stream: self,
            buffer: BytesMut::new(),
            last_read: 0
        }
    }
}

impl<I: AsRef<[u8]>, S: Stream<Item = I>> StreamEbml for S {}

impl<I: AsRef<[u8]>, S: Stream<Item = I>> EbmlStreamingParser<S> {
    pub fn poll_event<'a, T: FromEbml<'a>>(&'a mut self) -> Result<Async<Option<T>>, ParsingError<S::Error>> {
        // release buffer from previous event
        self.buffer.advance(self.last_read);
        self.last_read = 0;

        loop {
            match T::check_space(&self.buffer) {
                Ok(None) => {
                    // need to refill buffer, below
                },
                other => return other.map_err(ParsingError::EbmlError).and_then(move |_| {
                    match T::decode_element(&self.buffer) {
                        Err(err) => Err(ParsingError::EbmlError(err)),
                        Ok(None) => panic!("Buffer was supposed to have enough data to parse element, somehow did not."),
                        Ok(Some((element, element_size))) => {
                            self.last_read = element_size;
                            Ok(Async::Ready(Some(element)))
                        }
                    }
                })
            }

            match self.stream.poll().map_err(ParsingError::OtherError) {
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
