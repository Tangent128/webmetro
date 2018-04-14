use std::time::{Duration, Instant};

use futures::prelude::*;
use tokio::timer::Delay;

use chunk::Chunk;
use error::WebmetroError;

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

pub struct Throttle<S> {
    stream: S,
    start_time: Instant,
    sleep: Delay
}

impl<S: Stream<Item = Chunk, Error = WebmetroError>> Stream for Throttle<S>
{
    type Item = S::Item;
    type Error = WebmetroError;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, WebmetroError> {
        match self.sleep.poll() {
            Err(err) => return Err(WebmetroError::Unknown(Box::new(err))),
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Ok(Async::Ready(())) => { /* can continue */ }
        }

        let next_chunk = self.stream.poll();
        if let Ok(Async::Ready(Some(Chunk::ClusterHead(ref cluster_head)))) = next_chunk {
            // snooze until real time has "caught up" to the stream
            let offset = Duration::from_millis(cluster_head.end);
            self.sleep.reset(self.start_time + offset);
        }
        next_chunk
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

    fn throttle(self) -> Throttle<Self> {
        let now = Instant::now();
        Throttle {
            stream: self,
            start_time: now,
            sleep: Delay::new(now)
        }
    }
}

impl<T: Stream<Item = Chunk>> ChunkStream for T {}
