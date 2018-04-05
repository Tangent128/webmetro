use futures::Async;
use futures::stream::Stream;

use chunk::Chunk;

pub struct ChunkTimecodeFixer<S> {
    stream: S,
    current_offset: u64,
    last_observed_timecode: u64,
    assumed_duration: u64,
    seen_header: bool
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
            Ok(Async::Ready(Some(Chunk::Headers {..}))) => {
                if self.seen_header {
                    return self.poll();
                } else {
                    self.seen_header = true;
                }
            },
            _ => {}
        };
        poll_chunk
    }
}

pub trait ChunkStream<T> {
    fn fix_timecodes(self) -> ChunkTimecodeFixer<T>;
}

impl<T: Stream<Item = Chunk>> ChunkStream<T> for T {
    fn fix_timecodes(self) -> ChunkTimecodeFixer<T> {
        ChunkTimecodeFixer {
            stream: self,
            current_offset: 0,
            last_observed_timecode: 0,
            assumed_duration: 33,
            seen_header: false
        }
    }
}
