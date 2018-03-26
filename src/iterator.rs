use std::marker::PhantomData;
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
