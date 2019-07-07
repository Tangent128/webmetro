use futures::Async;

use crate::ebml::EbmlError;
use crate::ebml::EbmlEventSource;
use crate::ebml::FromEbml;

pub struct EbmlSlice<'a>(pub &'a [u8]);

impl<'b> EbmlEventSource for EbmlSlice<'b> {
    type Error = EbmlError;

    fn poll_event<'a, T: FromEbml<'a>>(&'a mut self) -> Result<Async<Option<T>>, EbmlError> {
        T::decode_element(self.0).map(|option| option.map(|(element, element_size)| {
            self.0 = &self.0[element_size..];
            element
        })).map(Async::Ready)
    }
}
