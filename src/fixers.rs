use std::pin::Pin;
use std::task::{Context, Poll};

use futures::prelude::*;
use pin_project::pin_project;
use tokio::time::{sleep_until, Duration, Instant, Sleep};

use crate::chunk::Chunk;

pub struct ChunkTimecodeFixer {
    current_offset: u64,
    last_observed_timecode: u64,
    assumed_duration: u64,
}

impl ChunkTimecodeFixer {
    pub fn new() -> ChunkTimecodeFixer {
        ChunkTimecodeFixer {
            current_offset: 0,
            last_observed_timecode: 0,
            assumed_duration: 33,
        }
    }
    pub fn process(&mut self, mut chunk: Chunk) -> Chunk {
        match chunk {
            Chunk::Cluster(ref mut cluster_head, _) => {
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
    seen_keyframe: bool,
}

impl<S: TryStream<Ok = Chunk> + Unpin> Stream for StartingPointFinder<S> {
    type Item = Result<Chunk, S::Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Chunk, S::Error>>> {
        loop {
            return match self.stream.try_poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(Chunk::Cluster(cluster_head, cluster_body)))) => {
                    if cluster_head.keyframe {
                        self.seen_keyframe = true;
                    }

                    if self.seen_keyframe {
                        Poll::Ready(Some(Ok(Chunk::Cluster(cluster_head, cluster_body))))
                    } else {
                        continue;
                    }
                }
                chunk @ Poll::Ready(Some(Ok(Chunk::Headers { .. }))) => {
                    if self.seen_header {
                        // new stream starting, we don't need a new header but should wait for a safe spot to resume
                        self.seen_keyframe = false;
                        continue;
                    } else {
                        self.seen_header = true;
                        chunk
                    }
                }
                chunk => chunk,
            };
        }
    }
}

#[pin_project]
pub struct Throttle<S> {
    #[pin]
    stream: S,
    start_time: Option<Instant>,
    #[pin]
    sleep: Sleep,
}

impl<S> Throttle<S> {
    pub fn new(wrap: S) -> Throttle<S> {
        let now = Instant::now();
        Throttle {
            stream: wrap,
            start_time: None,
            sleep: sleep_until(now),
        }
    }
}

impl<S: TryStream<Ok = Chunk> + Unpin> Stream for Throttle<S> {
    type Item = Result<Chunk, S::Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Chunk, S::Error>>> {
        let mut this = self.project();

        match this.sleep.as_mut().poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(()) => { /* can continue */ }
        }

        let next_chunk = this.stream.try_poll_next_unpin(cx);
        if let Poll::Ready(Some(Ok(Chunk::Cluster(ref cluster_head, _)))) = next_chunk {
            let offset = Duration::from_millis(cluster_head.end);
            // we have actual data, so start the clock if we haven't yet;
            // if we're starting the clock now, though, don't insert delays if the first chunk happens to start after zero
            let start_time = this
                .start_time
                .get_or_insert_with(|| Instant::now() - offset);
            // snooze until real time has "caught up" to the stream
            let sleep_until = *start_time + offset;
            this.sleep.reset(sleep_until);
        }
        next_chunk
    }
}

pub trait ChunkStream
where
    Self: Sized + TryStream<Ok = Chunk>,
{
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
            seen_keyframe: false,
        }
    }

    fn throttle(self) -> Throttle<Self> {
        Throttle::new(self)
    }
}

impl<T: TryStream<Ok = Chunk>> ChunkStream for T {}
