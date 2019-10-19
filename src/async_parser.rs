use bytes::{Bytes, BytesMut};
use std::future::Future;

use crate::ebml::FromEbml;
use crate::error::WebmetroError;

#[derive(Default)]
pub struct EbmlParser {
    buffer: BytesMut,
    buffer_size_limit: Option<usize>,
    borrowed: Bytes
}

impl EbmlParser {
    /// add a "soft" buffer size limit; if the input buffer exceeds this size,
    /// error the stream instead of resuming. It's still possible for the buffer
    /// to exceed this size *after* a fill, so ensure input sizes are reasonable.
    pub fn with_soft_limit(mut self, limit: usize) -> Self {
        self.buffer_size_limit = Some(limit);
        self
    }

    pub fn feed(&mut self, bytes: impl AsRef<[u8]>) {
        self.buffer.extend_from_slice(bytes.as_ref())
    }

    pub fn next_element<'a, T: FromEbml<'a>>(&'a mut self) -> Result<Option<T>, WebmetroError> {
        Ok(match T::check_space(&self.buffer)? {
            None => None,
            Some(info) => {
                let mut bytes = self.buffer.split_to(info.element_len).freeze();
                bytes.advance(info.body_offset);
                self.borrowed = bytes;
                Some(T::decode(info.element_id, &self.borrowed)?)
            }
        })
    }

    pub async fn next_element_with_feeder<
        'a,
        T: FromEbml<'a>,
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<Bytes, WebmetroError>>,
    >(
        &'a mut self,
        mut feeder: F,
    ) -> Result<Option<T>, WebmetroError> {
        loop {
            if let Some(_) = T::check_space(&self.buffer)? {
                return self.next_element();
            }

            if let Some(limit) = self.buffer_size_limit {
                if limit <= self.buffer.len() {
                    // hit our buffer limit and still nothing parsed
                    return Err(WebmetroError::ResourcesExceeded);
                }
            }

            self.buffer.extend(feeder().await?);
        }
    }
}

#[cfg(test)]
mod tests {
    use matches::assert_matches;

    use crate::async_parser::*;
    use crate::tests::ENCODE_WEBM_TEST_FILE;
    use crate::webm::*;

    #[test]
    fn async_webm_test() {
        let pieces = vec![
            &ENCODE_WEBM_TEST_FILE[0..20],
            &ENCODE_WEBM_TEST_FILE[20..40],
            &ENCODE_WEBM_TEST_FILE[40..],
        ];

        let mut piece_iter = pieces.iter();

        let result: Result<_, WebmetroError> = futures3::executor::block_on(async {
            let mut next = || {
                let result = if let Some(bytes) = piece_iter.next() {
                    Ok(Bytes::from(*bytes))
                } else {
                    Err("End of input".into())
                };
                async { result }
            };

            let mut parser = EbmlParser::default();

            assert_matches!(parser.next_element_with_feeder(&mut next).await?, Some(WebmElement::EbmlHead));
            assert_matches!(parser.next_element_with_feeder(&mut next).await?, Some(WebmElement::Segment));
            assert_matches!(parser.next_element_with_feeder(&mut next).await?, Some(WebmElement::Tracks(_)));
            assert_matches!(parser.next_element_with_feeder(&mut next).await?, Some(WebmElement::Cluster));
            assert_matches!(parser.next_element_with_feeder(&mut next).await?, Some(WebmElement::Timecode(0)));
            assert_matches!(parser.next_element_with_feeder(&mut next).await?, Some(WebmElement::SimpleBlock(_)));
            assert_matches!(parser.next_element_with_feeder(&mut next).await?, Some(WebmElement::Cluster));
            assert_matches!(parser.next_element_with_feeder(&mut next).await?, Some(WebmElement::Timecode(1000)));

            Ok(())
        });
        result.unwrap();
    }
}
