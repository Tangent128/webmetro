use bytes::{Buf, Bytes, BytesMut};
use futures::prelude::*;
use std::{
    io::Cursor,
    mem,
    pin::Pin,
    task::{Context, Poll, Poll::*},
};
use crate::stream_parser::EbmlStreamingParser;
use crate::error::WebmetroError;
use crate::webm::*;

#[derive(Clone, Debug)]
pub struct ClusterHead {
    pub keyframe: bool,
    pub start: u64,
    pub end: u64,
    /// a Cluster tag and a Timecode tag together take at most 15 bytes;
    /// fortuitously, 15 bytes can be inlined in a Bytes handle even on 32-bit systems
    bytes: BytesMut,
}

impl ClusterHead {
    pub fn new(timecode: u64) -> ClusterHead {
        let mut cluster_head = ClusterHead {
            keyframe: false,
            start: 0,
            end: 0,
            bytes: BytesMut::with_capacity(15),
        };
        cluster_head.update_timecode(timecode);
        cluster_head
    }
    pub fn update_timecode(&mut self, timecode: u64) {
        let delta = self.end - self.start;
        self.start = timecode;
        self.end = self.start + delta;
        let mut buffer = [0;15];
        let mut cursor = Cursor::new(buffer.as_mut());
        // buffer is sized so these should never fail
        encode_webm_element(WebmElement::Cluster, &mut cursor).unwrap();
        encode_webm_element(WebmElement::Timecode(timecode), &mut cursor).unwrap();
        self.bytes.clear();
        let len = cursor.position() as usize;
        self.bytes.extend_from_slice(&buffer[..len]);
    }
    pub fn observe_simpleblock_timecode(&mut self, timecode: i16) {
        let absolute_timecode = self.start + (timecode as u64);
        if absolute_timecode > self.start {
            self.end = absolute_timecode;
        }
    }
}

/// A chunk of WebM data
#[derive(Clone, Debug)]
pub enum Chunk {
    Headers {
        bytes: Bytes
    },
    ClusterHead(ClusterHead),
    ClusterBody {
        bytes: Bytes
    },
    Empty
}

pub struct Iter(Chunk);

impl Iterator for Chunk {
    type Item = Bytes;

    fn next(&mut self) -> Option<Bytes> {
        match self {
            Chunk::Headers {ref mut bytes, ..} => {
                let bytes = mem::replace(bytes, Bytes::new());
                *self = Chunk::Empty;
                Some(bytes)
            },
            Chunk::ClusterHead(ClusterHead {bytes, ..}) => {
                let bytes = mem::replace(bytes, BytesMut::new());
                *self = Chunk::Empty;
                Some(bytes.freeze())
            },
            Chunk::ClusterBody {bytes, ..} => {
                let bytes = mem::replace(bytes, Bytes::new());
                *self = Chunk::Empty;
                Some(bytes)
            },
            Chunk::Empty => None
        }
    }
}

#[derive(Debug)]
enum ChunkerState {
    BuildingHeader(Cursor<Vec<u8>>),
    // ClusterHead & body buffer
    BuildingCluster(ClusterHead, Cursor<Vec<u8>>),
    End
}

pub struct WebmChunker<S> {
    source: EbmlStreamingParser<S>,
    buffer_size_limit: Option<usize>,
    state: ChunkerState,
    pending_chunk: Option<Chunk>,
}

impl<S> WebmChunker<S> {
    /// add a "soft" buffer size limit; if a chunk buffer exceeds this size,
    /// error the stream instead of resuming. It's still possible for a buffer
    /// to exceed this size *after* a write, so ensure input sizes are reasonable.
    pub fn with_soft_limit(mut self, limit: usize) -> Self {
        self.buffer_size_limit = Some(limit);
        self
    }
}

fn encode(element: WebmElement, buffer: &mut Cursor<Vec<u8>>, limit: Option<usize>) -> Result<(), WebmetroError> {
    if let Some(limit) = limit {
        if limit <= buffer.get_ref().len() {
            return Err(WebmetroError::ResourcesExceeded);
        }
    }

    encode_webm_element(element, buffer).map_err(|err| err.into())
}

impl<I: Buf, E, S: Stream<Item = Result<I, E>> + Unpin> Stream for WebmChunker<S>
where
    WebmetroError: From<E>,
{
    type Item = Result<Chunk, WebmetroError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Result<Chunk, WebmetroError>>> {
        let mut chunker = self.get_mut();
        if chunker.pending_chunk.is_some() {
            return Ready(chunker.pending_chunk.take().map(Ok));
        }
        loop {
            match chunker.state {
                ChunkerState::BuildingHeader(ref mut buffer) => {
                    match chunker.source.poll_event(cx) {
                        Ready(Some(Err(passthru))) => return Ready(Some(Err(passthru))),
                        Pending => return Pending,
                        Ready(None) => return Ready(None),
                        Ready(Some(Ok(element))) => match element {
                            WebmElement::Cluster => {
                                let liberated_buffer = mem::replace(buffer, Cursor::new(Vec::new()));
                                let header_chunk = Chunk::Headers {bytes: Bytes::from(liberated_buffer.into_inner())};

                                chunker.state = ChunkerState::BuildingCluster(
                                    ClusterHead::new(0),
                                    Cursor::new(Vec::new())
                                );
                                return Ready(Some(Ok(header_chunk)));
                            },
                            WebmElement::Info => {},
                            WebmElement::Void => {},
                            WebmElement::Unknown(_) => {},
                            element => {
                                if let Err(err) = encode(element, buffer, chunker.buffer_size_limit) {
                                    chunker.state = ChunkerState::End;
                                    return Ready(Some(Err(err)));
                                }
                            }
                        }
                    }
                },
                ChunkerState::BuildingCluster(ref mut cluster_head, ref mut buffer) => {
                    match chunker.source.poll_event(cx) {
                        Ready(Some(Err(passthru))) => return Ready(Some(Err(passthru))),
                        Pending => return Pending,
                        Ready(Some(Ok(element))) => match element {
                            WebmElement::EbmlHead | WebmElement::Segment => {
                                let liberated_cluster_head = mem::replace(cluster_head, ClusterHead::new(0));
                                let liberated_buffer = mem::replace(buffer, Cursor::new(Vec::new()));

                                let mut new_header_cursor = Cursor::new(Vec::new());
                                match encode(element, &mut new_header_cursor, chunker.buffer_size_limit) {
                                    Ok(_) => {
                                        chunker.pending_chunk = Some(Chunk::ClusterBody {bytes: Bytes::from(liberated_buffer.into_inner())});
                                        chunker.state = ChunkerState::BuildingHeader(new_header_cursor);
                                        return Ready(Some(Ok(Chunk::ClusterHead(liberated_cluster_head))));
                                    },
                                    Err(err) => {
                                        chunker.state = ChunkerState::End;
                                        return Ready(Some(Err(err)));
                                    }
                                }
                            },
                            WebmElement::Cluster => {
                                let liberated_cluster_head = mem::replace(cluster_head, ClusterHead::new(0));
                                let liberated_buffer = mem::replace(buffer, Cursor::new(Vec::new()));

                                chunker.pending_chunk = Some(Chunk::ClusterBody {bytes: Bytes::from(liberated_buffer.into_inner())});
                                return Ready(Some(Ok(Chunk::ClusterHead(liberated_cluster_head))));
                            },
                            WebmElement::Timecode(timecode) => {
                                cluster_head.update_timecode(timecode);
                            },
                            WebmElement::SimpleBlock(ref block) => {
                                if (block.flags & 0b10000000) != 0 {
                                    // TODO: this is incorrect, condition needs to also affirm we're the first video block of the cluster
                                    cluster_head.keyframe = true;
                                }
                                cluster_head.observe_simpleblock_timecode(block.timecode);
                                if let Err(err) = encode(WebmElement::SimpleBlock(*block), buffer, chunker.buffer_size_limit) {
                                    chunker.state = ChunkerState::End;
                                    return Ready(Some(Err(err)));
                                }
                            },
                            WebmElement::Info => {},
                            WebmElement::Void => {},
                            WebmElement::Unknown(_) => {},
                            element => {
                                if let Err(err) = encode(element, buffer, chunker.buffer_size_limit) {
                                    chunker.state = ChunkerState::End;
                                    return Ready(Some(Err(err)));
                                }
                            },
                        },
                        Ready(None) => {
                            // flush final Cluster on end of stream
                            let liberated_cluster_head = mem::replace(cluster_head, ClusterHead::new(0));
                            let liberated_buffer = mem::replace(buffer, Cursor::new(Vec::new()));

                            chunker.pending_chunk = Some(Chunk::ClusterBody {bytes: Bytes::from(liberated_buffer.into_inner())});
                            chunker.state = ChunkerState::End;
                            return Ready(Some(Ok(Chunk::ClusterHead(liberated_cluster_head))));
                        }
                    }
                },
                ChunkerState::End => return Ready(None)
            };
        }
    }
}

pub trait WebmStream {
    type Stream;
    fn chunk_webm(self) -> WebmChunker<Self::Stream>;
}

impl<S: Stream> WebmStream for EbmlStreamingParser<S> {
    type Stream = S;
    fn chunk_webm(self) -> WebmChunker<S> {
        WebmChunker {
            source: self,
            buffer_size_limit: None,
            state: ChunkerState::BuildingHeader(Cursor::new(Vec::new())),
            pending_chunk: None
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::chunk::*;

    #[test]
    fn enough_space_for_header() {
        ClusterHead::new(u64::max_value());
    }
}
