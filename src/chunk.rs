use std::rc::Rc;

pub enum Chunk {
    Headers {
        bytes: Rc<Vec<u8>>
    },
    ClusterHead {
        keyframe: bool,
        start: u64,
        end: u64,
        // space for a Cluster tag and a Timecode tag
        bytes: [u8;16]
    },
    ClusterBody {
        bytes: Rc<Vec<u8>>
    }
}

impl AsRef<[u8]> for Chunk {
    fn as_ref(&self) -> &[u8] {
        match self {
            &Chunk::Headers {ref bytes, ..} => &bytes,
            &Chunk::ClusterHead {ref bytes, ..} => bytes,
            &Chunk::ClusterBody {ref bytes, ..} => &bytes
        }
    }
}

#[cfg(test)]
mod tests {

    use chunk::*;
    use std::io::Cursor;
    use webm::*;

    #[test]
    fn enough_space_for_header() {
        let mut chunk = Chunk::ClusterHead {
            keyframe: false,
            start: 0,
            end: 0,
            bytes: [0;16]
        };
        if let Chunk::ClusterHead {ref mut bytes, ..} = chunk {
            let mut cursor = Cursor::new(bytes as &mut [u8]);
            encode_webm_element(&WebmElement::Cluster, &mut cursor).unwrap();
            encode_webm_element(&WebmElement::Timecode(u64::max_value()), &mut cursor).unwrap();
        }
    }
}
