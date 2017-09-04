use std::io::{Cursor, Error as IoError, ErrorKind, Result as IoResult, Write, Seek};
use bytes::{BigEndian, BufMut, ByteOrder};
use ebml::*;

const SEGMENT_ID: u64 = 0x08538067;
const SEEK_HEAD_ID: u64 = 0x014D9B74;
const SEGMENT_INFO_ID: u64 = 0x0549A966;
const CUES_ID: u64 = 0x0C53BB6B;
const TRACKS_ID: u64 = 0x0654AE6B;
const CLUSTER_ID: u64 = 0x0F43B675;
const TIMECODE_ID: u64 = 0x67;
const SIMPLE_BLOCK_ID: u64 = 0x23;
pub struct Webm;

#[derive(Debug, PartialEq)]
pub struct SimpleBlock<'b> {
    pub track: u64,
    pub timecode: i16,
    pub flags: u8,
    pub data: &'b[u8]
}

#[derive(Debug, PartialEq)]
pub enum WebmElement<'b> {
    EbmlHead,
    Void,
    Segment,
    SeekHead,
    Info,
    Cues,
    Tracks(&'b[u8]),
    Cluster,
    Timecode(u64),
    SimpleBlock(SimpleBlock<'b>),
    Unknown(u64)
}

impl<'a> Schema<'a> for Webm {
    type Element = WebmElement<'a>;

    fn should_unwrap(&self, element_id: u64) -> bool {
        match element_id {
            // Segment
            SEGMENT_ID => true,
            CLUSTER_ID => true,
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
            CLUSTER_ID => Ok(WebmElement::Cluster),
            TIMECODE_ID => decode_uint(bytes).map(WebmElement::Timecode),
            SIMPLE_BLOCK_ID => decode_simple_block(bytes),
            _ => Ok(WebmElement::Unknown(element_id))
        }
    }
}

fn decode_simple_block(bytes: &[u8]) -> Result<WebmElement, Error> {
    if let Ok(Some((Varint::Value(track), track_field_len))) = decode_varint(bytes) {
        let header_len = track_field_len + 2 + 1;
        if bytes.len() < header_len {
            return Err(Error::CorruptPayload);
        }
        let timecode = BigEndian::read_i16(&bytes[track_field_len..]);
        let flags = bytes[track_field_len + 2];
        return Ok(WebmElement::SimpleBlock(SimpleBlock {
            track: track,
            timecode: timecode,
            flags: flags,
            data: &bytes[header_len..],
        }))
    } else {
        return Err(Error::CorruptPayload);
    }
}

pub fn encode_simple_block<T: Write>(block: SimpleBlock, output: &mut T) -> IoResult<()> {
    let SimpleBlock {
        track,
        timecode,
        flags,
        data
    } = block;

    // limiting number of tracks for now
    if track > 31 {
        return Err(IoError::new(ErrorKind::InvalidInput, WriteError::OutOfRange));
    }
    let header_len = 1 + 2 + 1;
    encode_tag_header(SIMPLE_BLOCK_ID, Varint::Value((header_len + data.len()) as u64), output)?;

    encode_varint(Varint::Value(track), output)?;

    let mut buffer = Cursor::new([0; 3]);
    buffer.put_i16::<BigEndian>(timecode);
    buffer.put_u8(flags);

    output.write_all(&buffer.get_ref()[..])?;
    output.write_all(data)
}

pub fn encode_webm_element<T: Write + Seek>(element: WebmElement, output: &mut T) -> IoResult<()> {
    match element {
        WebmElement::EbmlHead => encode_element(EBML_HEAD_ID, output, |output| {
            encode_bytes(DOC_TYPE_ID, "webm".as_bytes(), output)
        }),
        WebmElement::Segment => encode_tag_header(SEGMENT_ID, Varint::Unknown, output),
        WebmElement::SeekHead => Ok(()),
        WebmElement::Cues => Ok(()),
        WebmElement::Tracks(data) => encode_bytes(TRACKS_ID, data, output),
        WebmElement::Cluster => encode_tag_header(CLUSTER_ID, Varint::Unknown, output),
        WebmElement::Timecode(time) => encode_integer(TIMECODE_ID, time, output),
        WebmElement::SimpleBlock(block) => encode_simple_block(block, output),
        _ => Err(IoError::new(ErrorKind::InvalidInput, WriteError::OutOfRange))
    }
}

#[cfg(test)]
mod tests {
    use tests::TEST_FILE;
    use webm::*;

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

        assert_eq!(iter.next(), Some(WebmElement::Cluster));
        assert_eq!(iter.next(), Some(WebmElement::Timecode(0)));
        assert_eq!(iter.next(), Some(WebmElement::SimpleBlock(SimpleBlock {
            track: 1,
            timecode: 0,
            flags: 0b10000000,
            data: &TEST_FILE[443..3683]
        })));
        assert_eq!(iter.next(), Some(WebmElement::SimpleBlock(SimpleBlock {
            track: 1,
            timecode: 33,
            flags: 0b00000000,
            data: &TEST_FILE[3690..4735]
        })));
        assert_eq!(iter.next(), Some(WebmElement::SimpleBlock(SimpleBlock {
            track: 1,
            timecode: 67,
            flags: 0b00000000,
            data: &TEST_FILE[4741..4801]
        })));
        for _ in 3..30 {
            // skip remaining contents for brevity
            iter.next();
        }

        assert_eq!(iter.next(), Some(WebmElement::Cluster));
        assert_eq!(iter.next(), Some(WebmElement::Timecode(1000)));
        for _ in 0..30 {
            // skip contents for brevity
            iter.next();
        }

        assert_eq!(iter.next(), Some(WebmElement::Cluster));
        assert_eq!(iter.next(), Some(WebmElement::Timecode(2000)));
        for _ in 0..30 {
            // skip contents for brevity
            iter.next();
        }

        assert_eq!(iter.next(), Some(WebmElement::Cues));
        assert_eq!(iter.next(), None);
    }
}
