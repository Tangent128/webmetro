use futures::Async;
use ebml::EbmlError;
use ebml::EbmlEventSource;
use ebml::FromEbml;

pub struct EbmlSlice<'a>(pub &'a [u8]);

impl<'b> EbmlEventSource for EbmlSlice<'b> {
    type Error = EbmlError;

    fn poll_event<'a, T: FromEbml<'a>>(&'a mut self) -> Result<Async<Option<T>>, EbmlError> {
        T::check_space(self.0).and_then(|size_option| {
            match size_option {
                None => Ok(None),
                Some(element_size) => {
                    let (element_data, rest) = self.0.split_at(element_size);
                    self.0 = rest;
                    match T::decode_element(element_data) {
                        Err(err) => Err(err),
                        Ok(None) => panic!("Buffer was supposed to have enough data to parse element, somehow did not."),
                        Ok(Some((element, _))) => Ok(Some(element))
                    }
                }
            }
        }).map(Async::Ready)
    }
}
