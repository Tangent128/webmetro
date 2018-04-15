use clap::{App, AppSettings, ArgMatches, SubCommand};
use futures::prelude::*;

use super::stdin_stream;
use webmetro::{
    error::WebmetroError,
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

pub fn run(_args: &ArgMatches) -> Result<(), WebmetroError> {

    let mut events = stdin_stream().parse_ebml();

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
