
extern crate futures;

pub mod ebml;
mod iterator;
pub mod webm;

pub use ebml::{Error, Schema};

#[cfg(test)]
mod tests {

    use futures::future::{ok, Future};
    use super::*;
    use super::Error::{CorruptVarint, UnknownElementId};
    use super::Varint::{Unknown, Value};

    #[test]
    fn hello_futures() {
        let my_future = ok::<String, ()>("Hello".into())
            .map(|hello| hello + ", Futures!");

        let string_result = my_future.wait().unwrap();

        assert_eq!(string_result, "Hello, Futures!");
    }

    #[test]
    fn fail_corrupted_varints() {
        assert_eq!(decode_varint(&[0]), Err(CorruptVarint));
        assert_eq!(decode_varint(&[0, 0, 0]), Err(CorruptVarint));
    }

    #[test]
    fn incomplete_varints() {
        assert_eq!(decode_varint(&[]), Ok(None));
        assert_eq!(decode_varint(&[0x40]), Ok(None));
        assert_eq!(decode_varint(&[0x01, 0, 0]), Ok(None));
    }

    #[test]
    fn parse_varints() {
        assert_eq!(decode_varint(&[0xFF]), Ok(Some((Unknown, 1))));
        assert_eq!(decode_varint(&[0x7F, 0xFF]), Ok(Some((Unknown, 2))));
        assert_eq!(decode_varint(&[0x80]), Ok(Some((Value(0), 1))));
        assert_eq!(decode_varint(&[0x81]), Ok(Some((Value(1), 1))));
        assert_eq!(decode_varint(&[0x40, 52]), Ok(Some((Value(52), 2))));

        // test extra data in buffer
        assert_eq!(decode_varint(&[0x83, 0x11]), Ok(Some((Value(3), 1))));
    }

    #[test]
    fn fail_corrupted_tags() {
        assert_eq!(decode_tag(&[0]), Err(CorruptVarint));
        assert_eq!(decode_tag(&[0x80, 0]), Err(CorruptVarint));
        assert_eq!(decode_tag(&[0xFF, 0x80]), Err(UnknownElementId));
        assert_eq!(decode_tag(&[0x7F, 0xFF, 0x40, 0]), Err(UnknownElementId));
    }

    #[test]
    fn incomplete_tags() {
        assert_eq!(decode_tag(&[]), Ok(None));
        assert_eq!(decode_tag(&[0x80]), Ok(None));
        assert_eq!(decode_tag(&[0x40, 0, 0x40]), Ok(None));
    }

    #[test]
    fn parse_tags() {
        assert_eq!(decode_tag(&[0x80, 0x80]), Ok(Some((0, Value(0), 2))));
        assert_eq!(decode_tag(&[0x81, 0x85]), Ok(Some((1, Value(5), 2))));
        assert_eq!(decode_tag(&[0x80, 0xFF]), Ok(Some((0, Unknown, 2))));
        assert_eq!(decode_tag(&[0x80, 0x7F, 0xFF]), Ok(Some((0, Unknown, 3))));
        assert_eq!(decode_tag(&[0x85, 0x40, 52]), Ok(Some((5, Value(52), 3))));
    }

    const TEST_FILE: &'static [u8] = include_bytes!("data/test1.webm");

    struct Dummy;

    #[derive(Debug, PartialEq)]
    struct GenericElement(u64, usize);

    impl<'a> Schema<'a> for Dummy {
        type Element = GenericElement;

        fn should_unwrap(&self, element_id: u64) -> bool {
            match element_id {
                _ => false
            }
        }

        fn decode<'b: 'a>(&self, element_id: u64, bytes: &'b[u8]) -> Result<GenericElement, Error> {
            match element_id {
                _ => Ok(GenericElement(element_id, bytes.len()))
            }
        }
    }

    #[test]
    fn decode_sanity_test() {
        let decoded = Dummy.decode_element(TEST_FILE);
        assert_eq!(decoded, Ok(Some((GenericElement(0x0A45DFA3, 31), 43))));
    }

    #[test]
    fn decode_webm_test1() {
        let mut iter = Webm.parse(TEST_FILE).into_iter();

        // test that we match the structure of the test file
        assert_eq!(iter.next(), Some(WebmElement::EbmlHead));
        assert_eq!(iter.next(), Some(WebmElement::Segment));
        assert_eq!(iter.next(), Some(WebmElement::SeekHead));
        assert_eq!(iter.next(), Some(WebmElement::Void));
        assert_eq!(iter.next(), Some(WebmElement::Info));
        assert_eq!(iter.next(), Some(WebmElement::Tracks(&TEST_FILE[358..421])));
        assert_eq!(iter.next(), Some(WebmElement::Cluster(&TEST_FILE[433..13739])));
        assert_eq!(iter.next(), Some(WebmElement::Cluster(&TEST_FILE[13751..34814])));
        assert_eq!(iter.next(), Some(WebmElement::Cluster(&TEST_FILE[34826..56114])));
        assert_eq!(iter.next(), Some(WebmElement::Cues));
        assert_eq!(iter.next(), None);
    }

}
