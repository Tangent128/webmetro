use std::marker::PhantomData;

use ebml::FromEbml;

pub struct EbmlIterator<'a, T: FromEbml<'a>>(&'a [u8], PhantomData<fn() -> T>);

pub fn ebml_iter<'a, T: FromEbml<'a>>(source: &'a [u8])-> EbmlIterator<'a, T> {
    EbmlIterator(source, PhantomData)
}

impl<'a, T: FromEbml<'a>> Iterator for EbmlIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        T::decode_element(self.0).unwrap_or(None).and_then(|(element, element_size)| {
            self.0 = &self.0[element_size..];
            Some(element)
        })
    }
}
