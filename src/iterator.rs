use futures::Async;
use ebml::*;
use webm::*;

pub struct EbmlCursor<T> {
    source: T,
    position: usize
}

impl<T> EbmlCursor<T> {
    pub fn new(source: T) -> Self {
        EbmlCursor {
            source,
            position: 0
        }
    }
}

impl<'a> Iterator for EbmlCursor<&'a [u8]> {
    type Item = WebmElement<'a>;

    fn next(&mut self) -> Option<WebmElement<'a>> {
        match Self::Item::decode_element(&self.source[self.position..]) {
            Err(_) => None,
            Ok(None) => None,
            Ok(Some((element, element_size))) => {
                self.position += element_size;
                Some(element)
            }
        }
    }
}

impl<'b, T: AsRef<[u8]>> WebmEventSource for EbmlCursor<T> {
    type Error = Error;

    fn poll_event<'a>(&'a mut self) -> Result<Async<Option<WebmElement<'a>>>, Error> {
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
