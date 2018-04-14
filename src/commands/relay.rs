use std::error::Error;
use std::io::ErrorKind;
use std::net::ToSocketAddrs;
use std::sync::{
    Arc,
    Mutex
};

use clap::{App, Arg, ArgMatches, SubCommand};
use futures::{
    Future,
    Stream,
    Sink,
    future::{
        FutureResult,
        ok
    },
    stream::empty
};
use hyper::{
    Error as HyperError,
    Get,
    Head,
    Post,
    Put,
    StatusCode,
    header::ContentType,
    server::{Http, Request, Response, Service}
};
use webmetro::{
    channel::{
        Channel,
        Listener,
        Transmitter
    },
    chunk::{Chunk, WebmStream, ChunkingError},
    error::WebmetroError,
    fixers::ChunkStream,
    stream_parser::StreamEbml
};

type BodyStream = Box<Stream<Item = Chunk, Error = HyperError>>;

struct RelayServer(Arc<Mutex<Channel>>);

impl RelayServer {
    fn get_channel(&self) -> Arc<Mutex<Channel>> {
        self.0.clone()
    }

    fn get_stream(&self) -> BodyStream {
        Box::new(
            Listener::new(self.get_channel())
            .fix_timecodes()
            .find_starting_point()
            .map_err(|err| match err {})
        )
    }

    fn post_stream<I: AsRef<[u8]>, S: Stream<Item = I> + 'static>(&self, stream: S) -> BodyStream
    where S::Error: Error {
        let source = stream
            .map_err(|err| WebmetroError::Unknown(err.into()))
            .parse_ebml().chunk_webm();
        let sink = Transmitter::new(self.get_channel());

        Box::new(
            source.forward(sink.sink_map_err(|err| match err {}))
            .into_stream()
            .map(|_| empty())
            .map_err(|err| {
                let io_err = match err {
                    ChunkingError::IoError(io_err) => io_err,
                    ChunkingError::OtherError(_) => ErrorKind::InvalidData.into()
                };
                println!("Post failed: {}", &io_err);
                io_err
            })
            .flatten()
        )
    }
}

impl Service for RelayServer {
    type Request = Request;
    type Response = Response<BodyStream>;
    type Error = HyperError;
    type Future = FutureResult<Self::Response, HyperError>;

    fn call(&self, request: Request) -> Self::Future {
        let (method, uri, _http_version, _headers, request_body) = request.deconstruct();

        eprintln!("New {} Request: {}", method, uri.path());

        ok(match (method, uri.path()) {
            (Head, "/live") => {
                Response::new()
                    .with_header(ContentType("video/webm".parse().unwrap()))
            },
            (Get, "/live") => {
                Response::new()
                    .with_header(ContentType("video/webm".parse().unwrap()))
                    .with_body(self.get_stream())
            },
            (Post, "/live") | (Put, "/live") => {
                Response::new()
                    .with_body(self.post_stream(request_body))
            },
            _ => {
                Response::new()
                    .with_status(StatusCode::NotFound)
            }
        })
    }
}

pub fn options() -> App<'static, 'static> {
    SubCommand::with_name("relay")
        .about("Hosts an HTTP-based relay server")
        .arg(Arg::with_name("listen")
            .help("The address:port to listen to")
            .required(true))
}

pub fn run(args: &ArgMatches) -> Result<(), Box<Error>> {
    let single_channel = Channel::new();

    let addr_str = args.value_of("listen").ok_or("Listen address wasn't provided")?;
    let addr = addr_str.to_socket_addrs()?.next().ok_or("Listen address didn't resolve")?;

    Http::new().bind(&addr, move || Ok(RelayServer(single_channel.clone())))?.run()?;
    Ok(())
}
