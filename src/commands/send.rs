use bytes::Bytes;
use clap::{App, Arg, ArgMatches, SubCommand};
use futures::prelude::*;
use hyper::{client::HttpConnector, Body, Client, Request};
use std::io::{stdout, Write};
use stream::iter;

use super::{parse_time, stdin_stream};
use webmetro::{
    chunk::{Chunk, WebmStream},
    error::WebmetroError,
    fixers::{ChunkTimecodeFixer, Throttle},
    stream_parser::StreamEbml,
};

pub fn options() -> App<'static, 'static> {
    SubCommand::with_name("send")
        .about("PUTs WebM from stdin to a relay server.")
        .arg(Arg::with_name("url")
            .help("The location to upload to")
            .required(true))
        .arg(Arg::with_name("throttle")
            .long("throttle")
            .help("Slow down upload to \"real time\" speed as determined by the timestamps (useful for streaming static files)"))
        .arg(Arg::with_name("skip")
            .takes_value(true)
            .short("s")
            .long("skip")
            .help("Skip approximately n seconds of content before uploading or throttling"))
        .arg(Arg::with_name("take")
            .takes_value(true)
            .short("t")
            .long("take")
            .help("Stop uploading after approximately n seconds of content"))
}

type BoxedChunkStream = Box<dyn Stream<Item = Result<Chunk, WebmetroError>> + Send + Sync + Unpin>;

#[tokio::main]
pub async fn run(args: &ArgMatches) -> Result<(), WebmetroError> {
    // parse args
    let url_str = match args.value_of("url") {
        Some(url) => String::from(url),
        _ => return Err("Listen address wasn't provided".into()),
    };

    let start_time = parse_time(args.value_of("skip"))?.map_or(0, |s| s.as_millis());
    let stop_time = parse_time(args.value_of("take"))?.map_or(std::u128::MAX, |t| t.as_millis() + start_time);

    // build pipeline
    let mut timecode_fixer = ChunkTimecodeFixer::new();
    let mut chunk_stream: BoxedChunkStream = Box::new(
        stdin_stream()
            .parse_ebml()
            .chunk_webm()
            .map_ok(move |chunk| timecode_fixer.process(chunk))
            .try_filter(move |chunk| future::ready(chunk.overlaps(start_time, stop_time))),
    );

    if args.is_present("throttle") {
        chunk_stream = Box::new(Throttle::new(chunk_stream));
    }

    let chunk_stream = chunk_stream
        .map_ok(|webm_chunk| iter(webm_chunk).map(Result::<Bytes, WebmetroError>::Ok))
        .try_flatten()
        .map_err(|err| {
            warn!("{}", &err);
            err
        });

    let request_payload = Body::wrap_stream(chunk_stream);

    let request = Request::put(url_str).body(request_payload)?;
    let client = Client::builder().build(HttpConnector::new());

    let response = client.request(request).await?;
    let mut response_stream = response.into_body();
    while let Some(response_chunk) = response_stream.try_next().await? {
        stdout().write_all(&response_chunk)?;
    }
    Ok(())
}
