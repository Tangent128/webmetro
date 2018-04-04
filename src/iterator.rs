use std::marker::PhantomData;

use ebml::FromEbml;
use webm::WebmElement;

pub struct EbmlIterator<'a, T: FromEbml<'a>>(&'a [u8], PhantomData<fn() -> T>);

pub fn ebml_iter<'a, T: FromEbml<'a>>(source: &'a [u8])-> EbmlIterator<'a, T> {
    EbmlIterator(source, PhantomData)
}

impl<'a, T: FromEbml<'a>> Iterator for EbmlIterator<'a, T> {
    type Item = WebmElement<'a>;

    fn next(&mut self) -> Option<WebmElement<'a>> {
        WebmElement::check_space(self.0).unwrap_or(None).and_then(|element_size| {
            let (element_data, rest) = self.0.split_at(element_size);
            self.0 = rest;
            match WebmElement::decode_element(element_data) {
                Err(_) => None,
                Ok(None) => panic!("Buffer was supposed to have enough data to parse element, somehow did not."),
                Ok(Some((element, _))) => Some(element)
            }
        })
    }
}
