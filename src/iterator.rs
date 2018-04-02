use std::marker::PhantomData;
use futures::Async;
use ebml::*;

pub struct EbmlIterator<'b, T: FromEbml<'b>> {
    slice: &'b[u8],
    position: usize,
    _marker: PhantomData<fn() -> T>
}

impl<'b, E: FromEbml<'b>> IntoIterator for Ebml<&'b[u8], E> {
    type Item = E;
    type IntoIter = EbmlIterator<'b, E>;

    fn into_iter(self) -> EbmlIterator<'b, E>
    {
        EbmlIterator {
            slice: self.source,
            position: 0,
            _marker: PhantomData
        }
    }
}

impl<'b, T: FromEbml<'b>> Iterator for EbmlIterator<'b, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match Self::Item::decode_element(&self.slice[self.position..]) {
            Err(_) => None,
            Ok(None) => None,
            Ok(Some((element, element_size))) => {
                self.position += element_size;
                Some(element)
            }
        }
    }
}

impl<'a, T: FromEbml<'a>> EbmlEventSource<'a> for EbmlIterator<'a, T> {
    type Event = T;
    type Error = Error;

    fn poll_event(&'a mut self) -> Result<Async<Option<T>>, Error> {
        match Self::Event::decode_element(&self.slice[self.position..]) {
            Err(err) => Err(err),
            Ok(None) => Ok(Async::Ready(None)),
            Ok(Some((element, element_size))) => {
                self.position += element_size;
                Ok(Async::Ready(Some(element)))
            }
        }
    }
}
