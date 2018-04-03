use bytes::BytesMut;
use bytes::BufMut;
use futures::Async;
use futures::stream::Stream;

use ebml::*;
use webm::*;

pub enum ParsingError<E> {
    EbmlError(::ebml::Error),
    OtherError(E)
}

pub struct WebmBuffer<S> {
    stream: S,
    buffer: BytesMut,
    last_read: usize
}

impl<I: AsRef<[u8]>, S: Stream<Item = I>> WebmBuffer<S> {
    pub fn new(stream: S) -> Self {
        WebmBuffer {
            stream: stream,
            buffer: BytesMut::new(),
            last_read: 0
        }
    }

    pub fn try_decode(&mut self) -> Result<Async<Option<WebmElement>>, ParsingError<S::Error>> {
        match WebmElement::decode_element(&self.buffer) {
            Err(err) => return Err(ParsingError::EbmlError(err)),
            Ok(None) => {
                // need to refill buffer
                return Ok(Async::NotReady);
            },
            Ok(Some((element, element_size))) => {
                self.last_read += element_size;
                return Ok(Async::Ready(Some(element)))
            }
        };
    }

    pub fn can_decode(&mut self) -> bool {
        match self.try_decode() {
            Ok(Async::NotReady) => false,
            _ => true
        }
    }

    pub fn poll_event<'a>(&'a mut self) -> Result<Async<Option<WebmElement<'a>>>, ParsingError<S::Error>> {
        // release buffer from previous event
        self.buffer.advance(self.last_read);
        self.last_read = 0;

        loop {
            if self.can_decode() {
                return self.try_decode()
            }

            match self.stream.poll() {
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Ok(Async::Ready(None)) => return Ok(Async::Ready(None)),
                Ok(Async::Ready(Some(chunk))) => {
                    self.buffer.reserve(chunk.as_ref().len());
                    self.buffer.put_slice(chunk.as_ref());
                    // ok can retry decoding now
                }
                Err(err) => return Err(ParsingError::OtherError(err))
            };
        }
    }
}

impl<I: AsRef<[u8]>, S: Stream<Item = I>> WebmEventSource for WebmBuffer<S> {
    type Error = ParsingError<S::Error>;

    fn poll_event<'a>(&'a mut self) -> Result<Async<Option<WebmElement<'a>>>, Self::Error> {
        return WebmBuffer::poll_event(self);
    }
}

#[cfg(test)]
mod tests {
    //#[test]
    
}
