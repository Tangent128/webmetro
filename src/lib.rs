
extern crate futures;

#[derive(Debug, PartialEq)]
pub enum Error {
    CorruptVarint,
    UnknownElementId,
    UnknownElementLength,
    CorruptPayload,
}

#[derive(Debug, PartialEq)]
pub enum Varint {
    /// a numeric value
    Value(u64),
    /// the reserved "unknown" value
    Unknown
}

/// Try to parse an EBML varint at the start of the given slice.
/// Returns an Err() if the format is corrupt.
/// Returns Ok(None) if more bytes are needed to get a result.
/// Returns Ok(Some((varint, size))) to return a varint value and
/// the size of the parsed varint.
pub fn decode_varint(bytes: &[u8]) -> Result<Option<(Varint, usize)>, Error> {
    let mut value: u64 = 0;
    let mut value_length = 1;
    let mut mask: u8 = 0x80;
    let mut unknown_marker: u64 = !0;

    if bytes.len() == 0 {
        return Ok(None)
    }

    // get length marker bit from first byte & parse first byte
    while mask > 0 {
        if (mask & bytes[0]) != 0 {
            value = (bytes[0] & !mask) as u64;
            unknown_marker = (mask - 1) as u64;
            break
        }
        value_length += 1;
        mask = mask >> 1;
    }

    if mask == 0 {
        return Err(Error::CorruptVarint)
    }

    // check we have enough data to parse
    if value_length > bytes.len() {
        return Ok(None)
    }

    // decode remaining bytes
    for i in 1..value_length {
        value = (value << 8) + (bytes[i] as u64);
        unknown_marker = (unknown_marker << 8) + 0xFF;
    }

    // determine result
    if value == unknown_marker {
        Ok(Some((Varint::Unknown, value_length)))
    } else {
        Ok(Some((Varint::Value(value), value_length)))
    }
}

/// Try to parse an EBML element header at the start of the given slice.
/// Returns an Err() if the format is corrupt.
/// Returns Ok(None) if more bytes are needed to get a result.
/// Returns Ok(Some((id, varint, size))) to return the element id,
/// the size of the payload, and the size of the parsed header.
pub fn decode_tag(bytes: &[u8]) -> Result<Option<(u64, Varint, usize)>, Error> {
    // parse element ID
    match decode_varint(bytes) {
        Ok(None) => Ok(None),
        Err(err) => Err(err),
        Ok(Some((Varint::Unknown, _))) => Err(Error::UnknownElementId),
        Ok(Some((Varint::Value(element_id), id_size))) => {
            // parse payload size
            match decode_varint(&bytes[id_size..]) {
                Ok(None) => Ok(None),
                Err(err) => Err(err),
                Ok(Some((element_length, length_size))) =>
                    Ok(Some((
                        element_id,
                        element_length,
                        id_size + length_size
                    )))
            }
        }
    }
}

pub trait Schema<'a> {
    type Element: 'a;

    fn should_unwrap(&self, element_id: u64) -> bool;
    fn decode<'b: 'a>(&self, element_id: u64, bytes: &'b[u8]) -> Result<Self::Element, Error>;

    fn decode_element<'b: 'a>(&self, bytes: &'b[u8]) -> Result<Option<(Self::Element, usize)>, Error> {
        match decode_tag(bytes) {
            Ok(None) => Ok(None),
            Err(err) => Err(err),
            Ok(Some((element_id, payload_size_tag, tag_size))) => {
                let should_unwrap = self.should_unwrap(element_id);

                let payload_size = match (should_unwrap, payload_size_tag) {
                    (true, _) => 0,
                    (false, Varint::Unknown) => return Err(Error::UnknownElementLength),
                    (false, Varint::Value(size)) => size as usize
                };

                let element_size = tag_size + payload_size;
                if element_size > bytes.len() {
                    // need to read more still
                    return Ok(None);
                }

                match self.decode(element_id, &bytes[tag_size..element_size]) {
                    Ok(element) => Ok(Some((element, element_size))),
                    Err(error) => Err(error)
                }
            }
        }
    }

}

pub const EBML_HEAD_ID: u64 = 0x0A45DFA3;
pub const VOID_ID: u64 = 0x6C;

#[derive(Debug, PartialEq)]
pub struct Ebml<'b, S: Schema<'b>, T: 'b>(S, &'b T);

impl<'b, S: Schema<'b>> IntoIterator for Ebml<'b, S, &'b[u8]> {
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

const SEGMENT_ID: u64 = 0x08538067;
const SEEK_HEAD_ID: u64 = 0x014D9B74;
const SEGMENT_INFO_ID: u64 = 0x0549A966;
const CUES_ID: u64 = 0x0C53BB6B;
const TRACKS_ID: u64 = 0x0654AE6B;
const CLUSTER_ID: u64 = 0x0F43B675;
pub struct Webm;

#[derive(Debug, PartialEq)]
pub enum WebmElement<'b> {
    EbmlHead,
    Void,
    Segment,
    SeekHead,
    Info,
    Cues,
    Tracks(&'b[u8]),
    Cluster(&'b[u8]),
    Unknown(u64)
}

impl<'a> Schema<'a> for Webm {
    type Element = WebmElement<'a>;

    fn should_unwrap(&self, element_id: u64) -> bool {
        match element_id {
            // Segment
            SEGMENT_ID => true,
            _ => false
        }
    }

    fn decode<'b: 'a>(&self, element_id: u64, bytes: &'b[u8]) -> Result<WebmElement<'b>, Error> {
        match element_id {
            EBML_HEAD_ID => Ok(WebmElement::EbmlHead),
            VOID_ID => Ok(WebmElement::Void),
            SEGMENT_ID => Ok(WebmElement::Segment),
            SEEK_HEAD_ID => Ok(WebmElement::SeekHead),
            SEGMENT_INFO_ID => Ok(WebmElement::Info),
            CUES_ID => Ok(WebmElement::Cues),
            TRACKS_ID => Ok(WebmElement::Tracks(bytes)),
            CLUSTER_ID => Ok(WebmElement::Cluster(bytes)),
            _ => Ok(WebmElement::Unknown(element_id))
        }
    }
}

pub struct EbmlIterator<'b, T: Schema<'b>> {
    schema: T,
    slice: &'b[u8],
    position: usize,
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
        let source = &TEST_FILE;
        let mut iter = Ebml(Webm, source).into_iter();

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
