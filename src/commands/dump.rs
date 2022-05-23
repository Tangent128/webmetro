use clap::Args;

use super::stdin_stream;
use webmetro::{
    error::WebmetroError,
    stream_parser::StreamEbml,
    webm::{SimpleBlock, WebmElement::*},
};

/// Dumps WebM parsing events from parsing stdin
#[derive(Args, Debug)]
pub struct DumpArgs;

#[tokio::main]
pub async fn run(_args: DumpArgs) -> Result<(), WebmetroError> {
    let mut events = stdin_stream().parse_ebml();

    while let Some(element) = events.next().await? {
        match element {
            // suppress printing byte arrays
            Tracks(slice) => println!("Tracks[{}]", slice.len()),
            SimpleBlock(SimpleBlock { timecode, .. }) => println!("SimpleBlock@{}", timecode),
            other => println!("{:?}", other),
        }
    }
    Ok(())
}
