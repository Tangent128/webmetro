use std::{io, io::prelude::*};

use clap::{App, Arg, ArgMatches, SubCommand};
use futures::prelude::*;

use super::stdin_stream;
use webmetro::{
    chunk::{Chunk, WebmStream},
    error::WebmetroError,
    fixers::{ChunkTimecodeFixer, Throttle},
    stream_parser::StreamEbml,
};

pub fn options() -> App<'static, 'static> {
    SubCommand::with_name("filter")
        .about("Copies WebM from stdin to stdout, applying the same cleanup & stripping the relay server does.")
        .arg(Arg::with_name("throttle")
            .long("throttle")
            .help("Slow down output to \"real time\" speed as determined by the timestamps (useful for streaming static files)"))
}

#[tokio::main]
pub async fn run(args: &ArgMatches) -> Result<(), WebmetroError> {
    let mut timecode_fixer = ChunkTimecodeFixer::new();
    let mut chunk_stream: Box<dyn Stream<Item = Result<Chunk, WebmetroError>> + Send + Unpin> =
        Box::new(
            stdin_stream()
                .parse_ebml()
                .chunk_webm()
                .map_ok(move |chunk| timecode_fixer.process(chunk)),
        );

    if args.is_present("throttle") {
        chunk_stream = Box::new(Throttle::new(chunk_stream));
    }

    while let Some(chunk) = chunk_stream.next().await {
        chunk?.try_for_each(|buffer| io::stdout().write_all(&buffer))?;
    }
    Ok(())
}
