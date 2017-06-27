use ebml::*;

pub struct EbmlIterator<'b, T: Schema<'b>> {
    schema: T,
    slice: &'b[u8],
    position: usize,
}

impl<'b, S: Schema<'b>> IntoIterator for Ebml<S, &'b[u8]> {
    type Item = S::Element;
    type IntoIter = EbmlIterator<'b, S>;

    fn into_iter(self) -> EbmlIterator<'b, S>
    {
        EbmlIterator {
            schema: self.0,
            slice: self.1,
            position: 0
        }
    }
}

impl<'b, T: Schema<'b>> Iterator for EbmlIterator<'b, T> {
    type Item = T::Element;

    fn next(&mut self) -> Option<T::Element> {
        match self.schema.decode_element(&self.slice[self.position..]) {
            Err(_) => None,
            Ok(None) => None,
            Ok(Some((element, element_size))) => {
                self.position += element_size;
                Some(element)
            }
        }
    }
}
