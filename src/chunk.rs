use bytes::{Buf, Bytes};
use futures::{Async};
use futures3::prelude::*;
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
    /// space for a Cluster tag and a Timecode tag
    /// TODO: consider using a BytesMut here for simplicity
    bytes: [u8;16],
    bytes_used: u8
}

impl ClusterHead {
    pub fn new(timecode: u64) -> ClusterHead {
        let mut cluster_head = ClusterHead {
            keyframe: false,
            start: 0,
            end: 0,
            bytes: [0;16],
            bytes_used: 0
        };
        cluster_head.update_timecode(timecode);
        cluster_head
    }
    pub fn update_timecode(&mut self, timecode: u64) {
        let delta = self.end - self.start;
        self.start = timecode;
        self.end = self.start + delta;
        let mut cursor = Cursor::new(self.bytes.as_mut());
        // buffer is sized so these should never fail
        encode_webm_element(WebmElement::Cluster, &mut cursor).unwrap();
        encode_webm_element(WebmElement::Timecode(timecode), &mut cursor).unwrap();
        self.bytes_used = cursor.position() as u8;
    }
    pub fn observe_simpleblock_timecode(&mut self, timecode: i16) {
        let absolute_timecode = self.start + (timecode as u64);
        if absolute_timecode > self.start {
            self.end = absolute_timecode;
        }
    }
}

impl AsRef<[u8]> for ClusterHead {
    fn as_ref(&self) -> &[u8] {
        self.bytes[..self.bytes_used as usize].as_ref()
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
    }
}

impl Chunk {
    /// converts this chunk of data into a Bytes object, perhaps to send over the network
    pub fn into_bytes(self) -> Bytes {
        match self {
            Chunk::Headers {bytes, ..} => bytes,
            Chunk::ClusterHead(cluster_head) => Bytes::from(cluster_head.as_ref()),
            Chunk::ClusterBody {bytes, ..} => bytes
        }
    }
}

impl AsRef<[u8]> for Chunk {
    fn as_ref(&self) -> &[u8] {
        match self {
            &Chunk::Headers {ref bytes, ..} => bytes.as_ref(),
            &Chunk::ClusterHead(ref cluster_head) => cluster_head.as_ref(),
            &Chunk::ClusterBody {ref bytes, ..} => bytes.as_ref()
        }
    }
}

#[derive(Debug)]
enum ChunkerState {
    BuildingHeader(Cursor<Vec<u8>>),
    // ClusterHead & body buffer
    BuildingCluster(ClusterHead, Cursor<Vec<u8>>),
    EmittingClusterBody(Vec<u8>),
    EmittingClusterBodyBeforeNewHeader {
        body: Vec<u8>,
        new_header: Cursor<Vec<u8>>
    },
    EmittingFinalClusterBody(Vec<u8>),
    End
}

pub struct WebmChunker<S> {
    source: EbmlStreamingParser<S>,
    buffer_size_limit: Option<usize>,
    state: ChunkerState
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

impl<I: Buf, S: Stream<Item = Result<I, WebmetroError>> + Unpin> Stream for WebmChunker<S>
{
    type Item = Result<Chunk, WebmetroError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Result<Chunk, WebmetroError>>> {
        let mut chunker = self.get_mut();
        loop {
            match chunker.state {
                ChunkerState::BuildingHeader(ref mut buffer) => {
                    match chunker.source.poll_event(cx) {
                        Err(passthru) => return Ready(Some(Err(passthru))),
                        Ok(Async::NotReady) => return Pending,
                        Ok(Async::Ready(None)) => return Ready(None),
                        Ok(Async::Ready(Some(WebmElement::Cluster))) => {
                            let liberated_buffer = mem::replace(buffer, Cursor::new(Vec::new()));
                            let header_chunk = Chunk::Headers {bytes: Bytes::from(liberated_buffer.into_inner())};

                            chunker.state = ChunkerState::BuildingCluster(
                                ClusterHead::new(0),
                                Cursor::new(Vec::new())
                            );
                            return Ready(Some(Ok(header_chunk)));
                        },
                        Ok(Async::Ready(Some(WebmElement::Info))) => {},
                        Ok(Async::Ready(Some(WebmElement::Void))) => {},
                        Ok(Async::Ready(Some(WebmElement::Unknown(_)))) => {},
                        Ok(Async::Ready(Some(element))) => {
                            if let Err(err) = encode(element, buffer, chunker.buffer_size_limit) {
                                chunker.state = ChunkerState::End;
                                return Ready(Some(Err(err)));
                            }
                        }
                    }
                },
                ChunkerState::BuildingCluster(ref mut cluster_head, ref mut buffer) => {
                    match chunker.source.poll_event(cx) {
                        Err(passthru) => return Ready(Some(Err(passthru))),
                        Ok(Async::NotReady) => return Pending,
                        Ok(Async::Ready(Some(element @ WebmElement::EbmlHead)))
                        | Ok(Async::Ready(Some(element @ WebmElement::Segment))) => {
                            let liberated_cluster_head = mem::replace(cluster_head, ClusterHead::new(0));
                            let liberated_buffer = mem::replace(buffer, Cursor::new(Vec::new()));

                            let mut new_header_cursor = Cursor::new(Vec::new());
                            match encode(element, &mut new_header_cursor, chunker.buffer_size_limit) {
                                Ok(_) => {
                                    chunker.state = ChunkerState::EmittingClusterBodyBeforeNewHeader{
                                        body: liberated_buffer.into_inner(),
                                        new_header: new_header_cursor
                                    };
                                    return Ready(Some(Ok(Chunk::ClusterHead(liberated_cluster_head))));
                                },
                                Err(err) => {
                                    chunker.state = ChunkerState::End;
                                    return Ready(Some(Err(err)));
                                }
                            }
                        }
                        Ok(Async::Ready(Some(WebmElement::Cluster))) => {
                            let liberated_cluster_head = mem::replace(cluster_head, ClusterHead::new(0));
                            let liberated_buffer = mem::replace(buffer, Cursor::new(Vec::new()));

                            chunker.state = ChunkerState::EmittingClusterBody(liberated_buffer.into_inner());
                            return Ready(Some(Ok(Chunk::ClusterHead(liberated_cluster_head))));
                        },
                        Ok(Async::Ready(Some(WebmElement::Timecode(timecode)))) => {
                            cluster_head.update_timecode(timecode);
                        },
                        Ok(Async::Ready(Some(WebmElement::SimpleBlock(ref block)))) => {
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
                        Ok(Async::Ready(Some(WebmElement::Info))) => {},
                        Ok(Async::Ready(Some(WebmElement::Void))) => {},
                        Ok(Async::Ready(Some(WebmElement::Unknown(_)))) => {},
                        Ok(Async::Ready(Some(element))) => {
                            if let Err(err) = encode(element, buffer, chunker.buffer_size_limit) {
                                chunker.state = ChunkerState::End;
                                return Ready(Some(Err(err)));
                            }
                        },
                        Ok(Async::Ready(None)) => {
                            // flush final Cluster on end of stream
                            let liberated_cluster_head = mem::replace(cluster_head, ClusterHead::new(0));
                            let liberated_buffer = mem::replace(buffer, Cursor::new(Vec::new()));

                            chunker.state = ChunkerState::EmittingFinalClusterBody(liberated_buffer.into_inner());
                            return Ready(Some(Ok(Chunk::ClusterHead(liberated_cluster_head))));
                        }
                    }
                },
                ChunkerState::EmittingClusterBody(ref mut buffer) => {
                    let liberated_buffer = mem::replace(buffer, Vec::new());

                    chunker.state = ChunkerState::BuildingCluster(
                        ClusterHead::new(0),
                        Cursor::new(Vec::new())
                    );
                    return Ready(Some(Ok(Chunk::ClusterBody {bytes: Bytes::from(liberated_buffer)})));
                },
                ChunkerState::EmittingClusterBodyBeforeNewHeader { ref mut body, ref mut new_header } => {
                    let liberated_body = mem::replace(body, Vec::new());
                    let liberated_header_cursor = mem::replace(new_header, Cursor::new(Vec::new()));

                    chunker.state = ChunkerState::BuildingHeader(liberated_header_cursor);
                    return Ready(Some(Ok(Chunk::ClusterBody {bytes: Bytes::from(liberated_body)})));
                },
                ChunkerState::EmittingFinalClusterBody(ref mut buffer) => {
                    // flush final Cluster on end of stream
                    let liberated_buffer = mem::replace(buffer, Vec::new());

                    chunker.state = ChunkerState::End;
                    return Ready(Some(Ok(Chunk::ClusterBody {bytes: Bytes::from(liberated_buffer)})));
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
            state: ChunkerState::BuildingHeader(Cursor::new(Vec::new()))
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
