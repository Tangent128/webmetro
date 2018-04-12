use std::{
    error::Error,
    io::{self, prelude::*}
};

use clap::{App, AppSettings, ArgMatches, SubCommand};
use futures::{
    Async,
    stream::poll_fn
};

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
    let mut buf_reader = stdin.lock();
    let mut read_bytes = 0;

    let mut events = poll_fn(|| {
        buf_reader.consume(read_bytes);
        buf_reader.fill_buf().map(|slice| {
            read_bytes = slice.len();
            if read_bytes > 0 {
                Async::Ready(Some(Into::<Vec<u8>>::into(slice)))
            } else {
                Async::Ready(None)
            }
        })
    }).parse_ebml();

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
