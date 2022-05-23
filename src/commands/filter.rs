use std::{io, io::prelude::*, pin::Pin, time::Duration};

use clap::Args;
use futures::prelude::*;

use super::{parse_time, stdin_stream};
use webmetro::{
    chunk::{Chunk, WebmStream},
    error::WebmetroError,
    fixers::{ChunkTimecodeFixer, Throttle},
    stream_parser::StreamEbml,
};

/// Copies WebM from stdin to stdout, applying the same cleanup & stripping the relay server does.
#[derive(Args, Debug)]
pub struct FilterArgs {
    /// Slow down output to "real time" speed as determined by the timestamps (useful for streaming static files)
    #[clap(long)]
    throttle: bool,
    /// Skip approximately n seconds of content before uploading or throttling
    #[clap(long, short, parse(try_from_str = parse_time))]
    skip: Option<Duration>,
    /// Stop uploading after approximately n seconds of content
    #[clap(long, short, parse(try_from_str = parse_time))]
    take: Option<Duration>,
}

#[tokio::main]
pub async fn run(args: FilterArgs) -> Result<(), WebmetroError> {
    let start_time = args.skip.map_or(0, |s| s.as_millis());
    let stop_time = args
        .take
        .map_or(std::u128::MAX, |t| t.as_millis() + start_time);

    let mut timecode_fixer = ChunkTimecodeFixer::new();
    let mut chunk_stream: Pin<Box<dyn Stream<Item = Result<Chunk, WebmetroError>> + Send>> =
        Box::pin(
            stdin_stream()
                .parse_ebml()
                .chunk_webm()
                .map_ok(move |chunk| timecode_fixer.process(chunk))
                .try_filter(move |chunk| future::ready(chunk.overlaps(start_time, stop_time))),
        );

    if args.throttle {
        chunk_stream = Box::pin(Throttle::new(chunk_stream));
    }

    while let Some(chunk) = chunk_stream.next().await {
        chunk?.try_for_each(|buffer| io::stdout().write_all(&buffer))?;
    }
    Ok(())
}
