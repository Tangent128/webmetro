pub const EBML_HEAD_ID: u64 = 0x0A45DFA3;
pub const VOID_ID: u64 = 0x6C;

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

#[derive(Debug, PartialEq)]
pub struct Ebml<S, T>(pub S, pub T);

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

    fn parse<T>(self, source: T) -> Ebml<Self, T> where Self: Sized {
        Ebml(self, source)
    }
}
