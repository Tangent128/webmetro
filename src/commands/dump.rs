use clap::{App, AppSettings, ArgMatches, SubCommand};
use tokio2::runtime::Runtime;

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

    Runtime::new().unwrap().block_on(async {
        while let Some(element) = events.next().await? {
            match element {
                // suppress printing byte arrays
                Tracks(slice) => println!("Tracks[{}]", slice.len()),
                SimpleBlock(SimpleBlock {timecode, ..}) => println!("SimpleBlock@{}", timecode),
                other => println!("{:?}", other)
            }
        }
        Ok(())
    })
}
