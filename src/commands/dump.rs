use std::{
    error::Error,
    io
};

use clap::{App, AppSettings, ArgMatches, SubCommand};
use futures::Async;

use super::StdinStream;
use webmetro::{
    stream_parser::StreamEbml,
    webm::{
        SimpleBlock,
        WebmElement::*
    }
};

pub fn options() -> App<'static, 'static> {
    SubCommand::with_name("dump")
        .setting(AppSettings::Hidden)
        .about("Dumps WebM parsing events from parsing stdin")
}

pub fn run(_args: &ArgMatches) -> Result<(), Box<Error>> {

    let stdin = io::stdin();
    let mut events = StdinStream::new(stdin.lock()).parse_ebml();

    // stdin is sync so Async::NotReady will never happen
    while let Ok(Async::Ready(Some(element))) = events.poll_event() {
        match element {
            // suppress printing byte arrays
            Tracks(slice) => println!("Tracks[{}]", slice.len()),
            SimpleBlock(SimpleBlock {timecode, ..}) => println!("SimpleBlock@{}", timecode),
            other => println!("{:?}", other)
        }
    }

    Ok(())
}
