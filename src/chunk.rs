use futures::{Async, Stream};
use std::io::Cursor;
use std::mem;
use std::sync::Arc;
use webm::*;

#[derive(Clone, Debug)]
pub struct ClusterHead {
    pub keyframe: bool,
    pub start: u64,
    pub end: u64,
    // space for a Cluster tag and a Timecode tag
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
        encode_webm_element(&WebmElement::Cluster, &mut cursor).unwrap();
        encode_webm_element(&WebmElement::Timecode(timecode), &mut cursor).unwrap();
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

#[derive(Clone)]
pub enum Chunk<B: AsRef<[u8]> = Vec<u8>> {
    Headers {
        bytes: Arc<B>
    },
    ClusterHead(ClusterHead),
    ClusterBody {
        bytes: Arc<B>
    }
}

impl<B: AsRef<[u8]>> AsRef<[u8]> for Chunk<B> {
    fn as_ref(&self) -> &[u8] {
        match self {
            &Chunk::Headers {ref bytes, ..} => bytes.as_ref().as_ref(),
            &Chunk::ClusterHead(ref cluster_head) => cluster_head.as_ref(),
            &Chunk::ClusterBody {ref bytes, ..} => bytes.as_ref().as_ref()
        }
    }
}

#[derive(Debug)]
enum ChunkerState {
    BuildingHeader(Cursor<Vec<u8>>),
    // WIP ClusterHead & body buffer
    BuildingCluster(ClusterHead, Cursor<Vec<u8>>),
    EmittingClusterBody(Vec<u8>),
    EmittingFinalClusterBody(Vec<u8>),
    End
}

#[derive(Debug)]
pub enum ChunkingError<E> {
    IoError(::std::io::Error),
    OtherError(E)
}

pub struct WebmChunker<S: WebmEventSource> {
    source: S,
    state: ChunkerState
}

impl<'a, S: WebmEventSource> Stream for WebmChunker<S>
{
    type Item = Chunk;
    type Error = ChunkingError<S::Error>;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        loop {
            let (return_value, next_state) = match self.state {
                ChunkerState::BuildingHeader(ref mut buffer) => {
                    match self.source.poll_event() {
                        Err(passthru) => return Err(ChunkingError::OtherError(passthru)),
                        Ok(Async::NotReady) => return Ok(Async::NotReady),
                        Ok(Async::Ready(None)) => return Ok(Async::Ready(None)),
                        Ok(Async::Ready(Some(WebmElement::Cluster))) => {
                            let liberated_buffer = mem::replace(buffer, Cursor::new(Vec::new()));
                            let header_chunk = Chunk::Headers {bytes: Arc::new(liberated_buffer.into_inner())};
                            (
                                Ok(Async::Ready(Some(header_chunk))),
                                ChunkerState::BuildingCluster(
                                    ClusterHead::new(0),
                                    Cursor::new(Vec::new())
                                )
                            )
                        },
                        Ok(Async::Ready(Some(WebmElement::Info))) => continue,
                        Ok(Async::Ready(Some(WebmElement::Void))) => continue,
                        Ok(Async::Ready(Some(element @ _))) => {
                            match encode_webm_element(&element, buffer) {
                                Ok(_) => continue,
                                Err(err) => (
                                    Err(ChunkingError::IoError(err)),
                                    ChunkerState::End
                                )
                            }
                        }
                    }
                },
                ChunkerState::BuildingCluster(ref mut cluster_head, ref mut buffer) => {
                    match self.source.poll_event() {
                        Err(passthru) => return Err(ChunkingError::OtherError(passthru)),
                        Ok(Async::NotReady) => return Ok(Async::NotReady),
                        Ok(Async::Ready(Some(WebmElement::Cluster))) => {
                            let liberated_cluster_head = mem::replace(cluster_head, ClusterHead::new(0));
                            let liberated_buffer = mem::replace(buffer, Cursor::new(Vec::new()));
                            (
                                Ok(Async::Ready(Some(Chunk::ClusterHead(liberated_cluster_head)))),
                                ChunkerState::EmittingClusterBody(liberated_buffer.into_inner())
                            )
                        },
                        Ok(Async::Ready(Some(WebmElement::Timecode(timecode)))) => {
                            cluster_head.update_timecode(timecode);
                            continue;
                        },
                        Ok(Async::Ready(Some(WebmElement::SimpleBlock(ref block)))) => {
                            if (block.flags & 0b10000000) != 0 {
                                // TODO: this is incorrect, condition needs to also affirm we're the first video block of the cluster
                                cluster_head.keyframe = true;
                            }
                            cluster_head.observe_simpleblock_timecode(block.timecode);
                            match encode_webm_element(&WebmElement::SimpleBlock(*block), buffer) {
                                Ok(_) => continue,
                                Err(err) => (
                                    Err(ChunkingError::IoError(err)),
                                    ChunkerState::End
                                )
                            }
                        },
                        Ok(Async::Ready(Some(WebmElement::Info))) => continue,
                        Ok(Async::Ready(Some(WebmElement::Void))) => continue,
                        Ok(Async::Ready(Some(WebmElement::Unknown(_)))) => continue,
                        Ok(Async::Ready(Some(element @ _))) => {
                            match encode_webm_element(&element, buffer) {
                                Ok(_) => continue,
                                Err(err) => (
                                    Err(ChunkingError::IoError(err)),
                                    ChunkerState::End
                                )
                            }
                        },
                        Ok(Async::Ready(None)) => {
                            // flush final Cluster on end of stream
                            let liberated_cluster_head = mem::replace(cluster_head, ClusterHead::new(0));
                            let liberated_buffer = mem::replace(buffer, Cursor::new(Vec::new()));
                            (
                                Ok(Async::Ready(Some(Chunk::ClusterHead(liberated_cluster_head)))),
                                ChunkerState::EmittingFinalClusterBody(liberated_buffer.into_inner())
                            )
                        }
                    }
                },
                ChunkerState::EmittingClusterBody(ref mut buffer) => {
                    let liberated_buffer = mem::replace(buffer, Vec::new());
                    (
                        Ok(Async::Ready(Some(Chunk::ClusterBody {bytes: Arc::new(liberated_buffer)}))),
                        ChunkerState::BuildingCluster(
                            ClusterHead::new(0),
                            Cursor::new(Vec::new())
                        )
                    )
                },
                ChunkerState::EmittingFinalClusterBody(ref mut buffer) => {
                    // flush final Cluster on end of stream
                    let liberated_buffer = mem::replace(buffer, Vec::new());
                    (
                        Ok(Async::Ready(Some(Chunk::ClusterBody {bytes: Arc::new(liberated_buffer)}))),
                        ChunkerState::End
                    )
                },
                ChunkerState::End => return Ok(Async::Ready(None))
            };

            self.state = next_state;
            return return_value;
        }
    }
}

pub trait WebmStream<T: WebmEventSource> {
    fn chunk_webm(self) -> WebmChunker<T>;
}

impl<'a, T: WebmEventSource> WebmStream<T> for T {
    fn chunk_webm(self) -> WebmChunker<T> {
        WebmChunker {
            source: self,
            state: ChunkerState::BuildingHeader(Cursor::new(Vec::new()))
        }
    }
}

#[cfg(test)]
mod tests {

    use chunk::*;

    #[test]
    fn enough_space_for_header() {
        ClusterHead::new(u64::max_value());
    }
}
