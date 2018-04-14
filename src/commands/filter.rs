use std::{
    error::Error,
    io,
    io::prelude::*
};

use clap::{App, Arg, ArgMatches, SubCommand};
use futures::prelude::*;

use super::stdin_stream;
use webmetro::{
    chunk::{
        Chunk,
        WebmStream
    },
    error::WebmetroError,
    fixers::ChunkStream,
    stream_parser::StreamEbml
};

pub fn options() -> App<'static, 'static> {
    SubCommand::with_name("filter")
        .about("Copies WebM from stdin to stdout, applying the same cleanup & stripping the relay server does.")
        .arg(Arg::with_name("throttle")
            .long("throttle")
            .help("Slow down output to \"realtime\" speed as determined by the timestamps (useful for streaming)"))
}

pub fn run(args: &ArgMatches) -> Result<(), Box<Error>> {

    let mut chunk_stream: Box<Stream<Item = Chunk, Error = WebmetroError>> = Box::new(
        stdin_stream()
        .parse_ebml()
        .chunk_webm()
        .fix_timecodes()
    );

    if args.is_present("throttle") {
        chunk_stream = Box::new(chunk_stream.throttle());
    }

    chunk_stream.fold((), |_, chunk| {
        io::stdout().write_all(chunk.as_ref())
    }).wait()?;

    Ok(())
}
