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

impl<'a> Iterator for EbmlCursor<&'a [u8]> {
    type Item = WebmElement<'a>;

    fn next(&mut self) -> Option<WebmElement<'a>> {
        match Self::Item::decode_element(&self.source.as_ref()[self.position..]) {
            Err(_) => None,
            Ok(None) => None,
            Ok(Some((element, element_size))) => {
                self.position += element_size;
                Some(element)
            }
        }
    }
}

impl<'b, S: AsRef<[u8]>> WebmEventSource for EbmlCursor<S> {
    type Error = EbmlError;

    fn poll_event<'a>(&'a mut self) -> Result<Async<Option<WebmElement<'a>>>, EbmlError> {
        match WebmElement::decode_element(&self.source.as_ref()[self.position..]) {
            Err(err) => Err(err),
            Ok(None) => Ok(Async::Ready(None)),
            Ok(Some((element, element_size))) => {
                self.position += element_size;
                Ok(Async::Ready(Some(element)))
            }
        }
    }
}
