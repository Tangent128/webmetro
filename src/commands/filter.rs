use std::{
    error::Error,
    io,
    io::prelude::*
};

use clap::{App, Arg, ArgMatches, SubCommand};
use futures::Stream;

use super::StdinStream;
use webmetro::{
    chunk::{
        Chunk,
        WebmStream
    },
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

    let stdin = io::stdin();
    let mut chunk_stream: Box<Stream<Item = Chunk, Error = Box<Error>>> = Box::new(
        StdinStream::new(stdin.lock())
        .parse_ebml()
        .chunk_webm()
        .map_err(|err| Box::new(err) as Box<Error>)
        .fix_timecodes()
    );

    if args.is_present("throttle") {
        chunk_stream = Box::new(chunk_stream.throttle());
    }

    let stdout = io::stdout();
    let mut stdout_writer = stdout.lock();
    for chunk in chunk_stream.wait() {
        stdout_writer.write_all(chunk?.as_ref())?;
    }

    Ok(())
}
