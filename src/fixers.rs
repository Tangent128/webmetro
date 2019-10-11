use std::pin::Pin;
use std::task::{
    Context,
    Poll
};
use std::time::{Duration, Instant};

use futures3::prelude::*;
use tokio2::timer::{
    delay,
    Delay
};

use crate::chunk::Chunk;
use crate::error::WebmetroError;

pub struct ChunkTimecodeFixer {
    current_offset: u64,
    last_observed_timecode: u64,
    assumed_duration: u64
}

impl ChunkTimecodeFixer {
    pub fn new() -> ChunkTimecodeFixer {
        ChunkTimecodeFixer {
            current_offset: 0,
            last_observed_timecode: 0,
            assumed_duration: 33
        }
    }
    pub fn process<'a>(&mut self, mut chunk: Chunk) -> Chunk {
        match chunk {
            Chunk::ClusterHead(ref mut cluster_head) => {
                let start = cluster_head.start;
                if start < self.last_observed_timecode {
                    let next_timecode = self.last_observed_timecode + self.assumed_duration;
                    self.current_offset = next_timecode - start;
                }

                cluster_head.update_timecode(start + self.current_offset);
                self.last_observed_timecode = cluster_head.end;
            }
            _ => {}
        }
        chunk
    }
}

pub struct StartingPointFinder<S> {
    stream: S,
    seen_header: bool,
    seen_keyframe: bool
}

impl<S: TryStream<Ok = Chunk> + Unpin> Stream for StartingPointFinder<S>
{
    type Item = Result<Chunk, S::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Result<Chunk, S::Error>>> {
        loop {
            return match self.stream.try_poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(Chunk::ClusterHead(cluster_head)))) => {
                    if cluster_head.keyframe {
                        self.seen_keyframe = true;
                    }

                    if self.seen_keyframe {
                        Poll::Ready(Some(Ok(Chunk::ClusterHead(cluster_head))))
                    } else {
                        continue;
                    }
                },
                chunk @ Poll::Ready(Some(Ok(Chunk::ClusterBody {..}))) => {
                    if self.seen_keyframe {
                        chunk
                    } else {
                        continue;
                    }
                },
                chunk @ Poll::Ready(Some(Ok(Chunk::Headers {..}))) => {
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

impl<S: TryStream<Ok = Chunk, Error = WebmetroError> + Unpin> Stream for Throttle<S>
{
    type Item = Result<Chunk, WebmetroError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Result<Chunk, WebmetroError>>> {
        match self.sleep.poll_unpin(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(()) => { /* can continue */ },
        }

        let next_chunk = self.stream.try_poll_next_unpin(cx);
        if let Poll::Ready(Some(Ok(Chunk::ClusterHead(ref cluster_head)))) = next_chunk {
            // snooze until real time has "caught up" to the stream
            let offset = Duration::from_millis(cluster_head.end);
            let sleep_until = self.start_time + offset;
            self.sleep.reset(sleep_until);
        }
        next_chunk
    }
}

pub trait ChunkStream where Self : Sized + TryStream<Ok = Chunk> {
    /*fn fix_timecodes(self) -> Map<_> {
        let fixer = ;
        self.map(move |chunk| {
            fixer.process(chunk);
            chunk
        })
    }*/

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
            sleep: delay(now)
        }
    }
}

impl<T: TryStream<Ok = Chunk>> ChunkStream for T {}
