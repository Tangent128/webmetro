use std::{io, io::prelude::*, pin::Pin};

use clap::Args;
use futures::prelude::*;

use super::stdin_stream;
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
    #[clap(long, short)]
    throttle: bool,
}

#[tokio::main]
pub async fn run(args: FilterArgs) -> Result<(), WebmetroError> {
    let mut timecode_fixer = ChunkTimecodeFixer::new();
    let mut chunk_stream: Pin<Box<dyn Stream<Item = Result<Chunk, WebmetroError>> + Send>> =
        Box::pin(
            stdin_stream()
                .parse_ebml()
                .chunk_webm()
                .map_ok(move |chunk| timecode_fixer.process(chunk)),
        );

    if args.throttle {
        chunk_stream = Box::pin(Throttle::new(chunk_stream));
    }

    while let Some(chunk) = chunk_stream.next().await {
        chunk?.try_for_each(|buffer| io::stdout().write_all(&buffer))?;
    }
    Ok(())
}
