use clap::{App, Arg, ArgMatches, SubCommand};
use futures::{
    prelude::*
};
use hyper::{
    Body,
    Client,
    client::HttpConnector,
    Request
};
use tokio::runtime::Runtime;

use super::{
    stdin_stream
};
use webmetro::{
    chunk::{
        Chunk,
        WebmStream
    },
    error::WebmetroError,
    fixers::ChunkStream,
    stream_parser::StreamEbml
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
}

type BoxedChunkStream = Box<dyn Stream<Item = Chunk, Error = WebmetroError> + Send>;

pub fn run(args: &ArgMatches) -> Result<(), WebmetroError> {
    let mut chunk_stream: BoxedChunkStream = Box::new(
        stdin_stream()
        .parse_ebml()
        .chunk_webm()
        .fix_timecodes()
    );

    let url_str = match args.value_of("url") {
        Some(url) => String::from(url),
        _ => return Err("Listen address wasn't provided".into())
    };

    if args.is_present("throttle") {
        chunk_stream = Box::new(chunk_stream.throttle());
    }

    let request_payload = Body::wrap_stream(chunk_stream.map(
        |webm_chunk| webm_chunk.into_bytes()
    ).map_err(|err| {
        eprintln!("{}", &err);
        err
    }));

    
    let request = Request::put(url_str)
    .body(request_payload)
    .map_err(WebmetroError::from)?;

    let client = Client::builder().build(HttpConnector::new(1));
    let future = client.request(request)
    .and_then(|response| {
        response.into_body().for_each(|_chunk| {
            Ok(())
        })
    })
    .map_err(WebmetroError::from);

    Runtime::new().unwrap().block_on(future)
}
