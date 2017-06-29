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
    SimpleBlock(&'b[u8]),
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
            TIMECODE_ID => Ok(WebmElement::Timecode(0)),
            SIMPLE_BLOCK_ID => Ok(WebmElement::SimpleBlock(bytes)),
            _ => Ok(WebmElement::Unknown(element_id))
        }
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
        assert_eq!(iter.next(), Some(WebmElement::Cluster));
        assert_eq!(iter.next(), Some(WebmElement::Cluster));
        assert_eq!(iter.next(), Some(WebmElement::Cues));
        assert_eq!(iter.next(), None);
    }
}
