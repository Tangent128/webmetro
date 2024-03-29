use bytes::Bytes;
use clap::Args;
use futures::prelude::*;
use hyper::{client::HttpConnector, Body, Client, Request};
use std::{
    io::{stdout, Write},
    pin::Pin,
    time::Duration,
};
use stream::iter;

use super::{parse_time, stdin_stream};
use webmetro::{
    chunk::{Chunk, WebmStream},
    error::WebmetroError,
    fixers::{ChunkTimecodeFixer, Throttle},
    stream_parser::StreamEbml,
};

type BoxedChunkStream = Pin<Box<dyn Stream<Item = Result<Chunk, WebmetroError>> + Send + Sync>>;

/// PUTs WebM from stdin to a relay server.
#[derive(Args, Debug)]
pub struct SendArgs {
    /// The location to upload to
    url: String,
    /// Slow down upload to "real time" speed as determined by the timestamps (useful for streaming static files)
    #[clap(long)]
    throttle: bool,
    /// Skip approximately n seconds of content before uploading or throttling
    #[clap(long, short, parse(try_from_str = parse_time))]
    skip: Option<Duration>,
    /// Stop uploading after approximately n seconds of content
    #[clap(long, short, parse(try_from_str = parse_time))]
    take: Option<Duration>,
}

#[tokio::main]
pub async fn run(args: SendArgs) -> Result<(), WebmetroError> {
    let start_time = args.skip.map_or(0, |s| s.as_millis());
    let stop_time = args
        .take
        .map_or(std::u128::MAX, |t| t.as_millis() + start_time);

    // build pipeline
    let mut timecode_fixer = ChunkTimecodeFixer::new();
    let mut chunk_stream: BoxedChunkStream = Box::pin(
        stdin_stream()
            .parse_ebml()
            .chunk_webm()
            .map_ok(move |chunk| timecode_fixer.process(chunk))
            .try_filter(move |chunk| future::ready(chunk.overlaps(start_time, stop_time))),
    );

    if args.throttle {
        chunk_stream = Box::pin(Throttle::new(chunk_stream));
    }

    let chunk_stream = chunk_stream
        .map_ok(|webm_chunk| iter(webm_chunk).map(Result::<Bytes, WebmetroError>::Ok))
        .try_flatten()
        .map_err(|err| {
            warn!("{}", &err);
            err
        });

    let request_payload = Body::wrap_stream(chunk_stream);

    let request = Request::put(args.url).body(request_payload)?;
    let client = Client::builder().build(HttpConnector::new());

    let response = client.request(request).await?;
    let mut response_stream = response.into_body();
    while let Some(response_chunk) = response_stream.try_next().await? {
        stdout().write_all(&response_chunk)?;
    }
    Ok(())
}
