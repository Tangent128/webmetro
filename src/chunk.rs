use std::io::Cursor;
use std::sync::Arc;
use webm::*;

#[derive(Clone)]
pub enum Chunk<B: AsRef<[u8]> = Vec<u8>> {
    Headers {
        bytes: Arc<B>
    },
    ClusterHead {
        keyframe: bool,
        start: u64,
        end: u64,
        // space for a Cluster tag and a Timecode tag
        bytes: [u8;16],
        bytes_used: u8
    },
    ClusterBody {
        bytes: Arc<B>
    }
}

impl<B: AsRef<[u8]>> Chunk<B> {
    pub fn update_timecode(&mut self, timecode: u64) {
        if let &mut Chunk::ClusterHead {ref mut start, ref mut end, ref mut bytes, ref mut bytes_used, ..} = self {
            let delta = *end - *start;
            *start = timecode;
            *end = *start + delta;
            let mut cursor = Cursor::new(bytes as &mut [u8]);
            // buffer is sized so these should never fail
            encode_webm_element(&WebmElement::Cluster, &mut cursor).unwrap();
            encode_webm_element(&WebmElement::Timecode(timecode), &mut cursor).unwrap();
            *bytes_used = cursor.position() as u8;
        }
    }
}

impl<B: AsRef<[u8]>> AsRef<[u8]> for Chunk<B> {
    fn as_ref(&self) -> &[u8] {
        match self {
            &Chunk::Headers {ref bytes, ..} => bytes.as_ref().as_ref(),
            &Chunk::ClusterHead {ref bytes, bytes_used, ..} => bytes[..bytes_used as usize].as_ref(),
            &Chunk::ClusterBody {ref bytes, ..} => bytes.as_ref().as_ref()
        }
    }
}

#[cfg(test)]
mod tests {

    use chunk::*;

    #[test]
    fn enough_space_for_header() {
        let mut chunk: Chunk = Chunk::ClusterHead {
            keyframe: false,
            start: 0,
            end: 0,
            bytes: [0;16]
        };
        chunk.update_timecode(u64::max_value());
    }
}
