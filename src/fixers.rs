use futures::Async;
use futures::stream::Stream;

use chunk::Chunk;

pub struct ChunkTimecodeFixer<S> {
    stream: S,
    current_offset: u64,
    last_observed_timecode: u64,
    assumed_duration: u64
}

impl<S: Stream<Item = Chunk>> Stream for ChunkTimecodeFixer<S>
{
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        let mut poll_chunk = self.stream.poll();
        match poll_chunk {
            Ok(Async::Ready(Some(Chunk::ClusterHead(ref mut cluster_head)))) => {
                let start = cluster_head.start;
                if start < self.last_observed_timecode {
                    let next_timecode = self.last_observed_timecode + self.assumed_duration;
                    self.current_offset = next_timecode - start;
                }

                cluster_head.update_timecode(start + self.current_offset);
                self.last_observed_timecode = cluster_head.end;
            },
            _ => {}
        };
        poll_chunk
    }
}

pub struct StartingPointFinder<S> {
    stream: S,
    seen_header: bool,
    seen_keyframe: bool
}

impl<S: Stream<Item = Chunk>> Stream for StartingPointFinder<S>
{
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        loop {
            return match self.stream.poll() {
                Ok(Async::Ready(Some(Chunk::ClusterHead(cluster_head)))) => {
                    if cluster_head.keyframe {
                        self.seen_keyframe = true;
                    }

                    if self.seen_keyframe {
                        Ok(Async::Ready(Some(Chunk::ClusterHead(cluster_head))))
                    } else {
                        continue;
                    }
                },
                chunk @ Ok(Async::Ready(Some(Chunk::ClusterBody {..}))) => {
                    if self.seen_keyframe {
                        chunk
                    } else {
                        continue;
                    }
                },
                chunk @ Ok(Async::Ready(Some(Chunk::Headers {..}))) => {
                    if self.seen_header {
                        // new stream starting, we don't need a new header but should wait for a safe spot to resume
                        self.seen_keyframe = false;
                        continue;
                    } else {
                        self.seen_header = true;
                        chunk
                    }
                },
                chunk => chunk
            }
        };
    }
}

pub trait ChunkStream where Self : Sized + Stream<Item = Chunk> {
    fn fix_timecodes(self) -> ChunkTimecodeFixer<Self> {
        ChunkTimecodeFixer {
            stream: self,
            current_offset: 0,
            last_observed_timecode: 0,
            assumed_duration: 33
        }
    }

    fn find_starting_point(self) -> StartingPointFinder<Self> {
        StartingPointFinder {
            stream: self,
            seen_header: false,
            seen_keyframe: false
        }
    }
}

impl<T: Stream<Item = Chunk>> ChunkStream for T {}
