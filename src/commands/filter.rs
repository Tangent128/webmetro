use std::{
    error::Error,
    io,
    io::prelude::*
};

use clap::{App, ArgMatches, SubCommand};
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
}

pub fn run(_args: &ArgMatches) -> Result<(), Box<Error>> {

    let stdin = io::stdin();
    let chunk_stream: Box<Stream<Item = Chunk, Error = Box<Error>>> = Box::new(
        StdinStream::new(stdin.lock())
        .parse_ebml()
        .chunk_webm()
        .map_err(|err| Box::new(err) as Box<Error>)
        .fix_timecodes()
    );

    let stdout = io::stdout();
    let mut stdout_writer = stdout.lock();
    for chunk in chunk_stream.wait() {
        stdout_writer.write_all(chunk?.as_ref())?;
    }

    Ok(())
}
