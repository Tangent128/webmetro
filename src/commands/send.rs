use clap::{App, Arg, ArgMatches, SubCommand};
use futures::{
    future,
    prelude::*
};
use hyper::{
    Error as HyperError,
    Method,
    client::{
        Config,
        Request
    }
};
use tokio_core::reactor::{
    Handle
};

use super::{
    stdin_stream,
    to_hyper_error
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

type BoxedChunkStream = Box<Stream<Item = Chunk, Error = WebmetroError>>;
type BoxedHyperStream = Box<Stream<Item = Chunk, Error = HyperError>>;

pub fn run(handle: Handle, args: &ArgMatches) -> Box<Future<Item=(), Error=WebmetroError>> {
    let mut chunk_stream: BoxedChunkStream = Box::new(
        stdin_stream()
        .parse_ebml()
        .chunk_webm()
        .fix_timecodes()
    );

    let url_str = match args.value_of("url") {
        Some(url) => String::from(url),
        _ => return Box::new(Err(WebmetroError::from_str("Listen address wasn't provided")).into_future())
    };

    if args.is_present("throttle") {
        chunk_stream = Box::new(chunk_stream.throttle());
    }

    let request_body_stream = Box::new(chunk_stream.map_err(|err| {
        eprintln!("{}", &err);
        to_hyper_error(err)
    })) as BoxedHyperStream;

    Box::new(future::lazy(move || {
        url_str.parse().map_err(WebmetroError::from_err)
    }).and_then(move |uri| {
        let client = Config::default()
            .body::<BoxedHyperStream>()
            .build(&handle);

        let mut request: Request<BoxedHyperStream> = Request::new(Method::Put, uri);
        request.set_body(request_body_stream);

        client.request(request)
            .and_then(|response| {
                response.body().for_each(|_chunk| {
                    Ok(())
                })
            })
            .map_err(WebmetroError::from_err)
    }))
}
