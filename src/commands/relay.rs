use std::error::Error;
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
    chunk::{Chunk, WebmStream},
    error::WebmetroError,
    fixers::ChunkStream,
    stream_parser::StreamEbml
};

use super::to_hyper_error;

const BUFFER_LIMIT: usize = 2 * 1024 * 1024;

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
    where S::Error: Error + Send {
        let source = stream
            .map_err(WebmetroError::from_err)
            .parse_ebml().with_soft_limit(BUFFER_LIMIT)
            .chunk_webm().with_soft_limit(BUFFER_LIMIT);
        let sink = Transmitter::new(self.get_channel());

        Box::new(
            source.forward(sink.sink_map_err(|err| -> WebmetroError {match err {}}))
            .into_stream()
            .map(|_| empty())
            .map_err(|err| {
                //TODO: log something somewhere
                to_hyper_error(err)
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

        //TODO: log equiv to: eprintln!("New {} Request: {}", method, uri.path());

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

pub fn run(args: &ArgMatches) -> Result<(), WebmetroError> {
    let single_channel = Channel::new();

    let addr_str = args.value_of("listen").ok_or("Listen address wasn't provided")?;
    let addr = addr_str.to_socket_addrs()?.next().ok_or("Listen address didn't resolve")?;

    Http::new()
        .bind(&addr, move || {
            Ok(RelayServer(single_channel.clone()))
        })
        .map_err(|err| WebmetroError::Unknown(Box::new(err)))?
        .run()
        .map_err(|err| WebmetroError::Unknown(Box::new(err)))?;

    Ok(())
}
