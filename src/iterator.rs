use futures::Async;
use ebml::Error as EbmlError;
use ebml::FromEbml;
use webm::*;

pub struct EbmlCursor<T> {
    source: T,
    position: usize
}

impl<S> EbmlCursor<S> {
    pub fn new(source: S) -> Self {
        EbmlCursor {
            source,
            position: 0
        }
    }
}

impl<'a> EbmlCursor<&'a [u8]> {
    fn decode_element<T: FromEbml<'a>>(&mut self) -> Result<Option<T>, EbmlError> {
        match T::decode_element(&self.source.as_ref()[self.position..]) {
            Err(err) => Err(err),
            Ok(None) => Ok(None),
            Ok(Some((element, element_size))) => {
                self.position += element_size;
                Ok(Some(element))
            }
        }
    }
}

impl<'a> Iterator for EbmlCursor<&'a [u8]> {
    type Item = WebmElement<'a>;

    fn next(&mut self) -> Option<WebmElement<'a>> {
        self.decode_element().unwrap_or(None)
    }
}

impl<'b> WebmEventSource for EbmlCursor<&'b [u8]> {
    type Error = EbmlError;

    fn poll_event<'a>(&'a mut self) -> Result<Async<Option<WebmElement<'a>>>, EbmlError> {
        self.decode_element().map(Async::Ready)
    }
}
