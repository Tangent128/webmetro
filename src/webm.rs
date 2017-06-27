use ebml::*;

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
