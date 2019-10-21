use clap::{App, AppSettings, ArgMatches, SubCommand};
use futures::Async;
use futures3::future::{FutureExt, poll_fn};
use std::task::Poll;

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

    Ok(poll_fn(|cx| {
        // stdin is sync so Async::NotReady will never happen on this tokio version
        while let Ok(Async::Ready(Some(element))) = events.poll_event(cx) {
            match element {
                // suppress printing byte arrays
                Tracks(slice) => println!("Tracks[{}]", slice.len()),
                SimpleBlock(SimpleBlock {timecode, ..}) => println!("SimpleBlock@{}", timecode),
                other => println!("{:?}", other)
            }
        }

        Poll::Ready(())
    }).now_or_never().expect("Stdin should never go async"))
}
