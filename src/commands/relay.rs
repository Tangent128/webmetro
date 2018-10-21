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
use http::{
    request::Parts,
    StatusCode,
};
use hyper::{
    Body,
    Method,
    Request,
    Response,
    rt,
    Server,
    service::Service,
    header::{
        CACHE_CONTROL,
        CONTENT_TYPE
    }
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

use super::WebmPayload;

const BUFFER_LIMIT: usize = 2 * 1024 * 1024;

struct RelayServer(Arc<Mutex<Channel>>);

impl RelayServer {
    fn get_channel(&self) -> Arc<Mutex<Channel>> {
        self.0.clone()
    }

    fn get_stream(&self) -> impl Stream<Item = Chunk, Error = WebmetroError> {
        Listener::new(self.get_channel())
        .fix_timecodes()
        .find_starting_point()
        .map_err(|err| match err {})
    }

    fn post_stream<I: AsRef<[u8]>, S: Stream<Item = I> + Send + 'static>(&self, stream: S) -> impl Stream<Item = Chunk, Error = WebmetroError>
    where S::Error: Error + Send + Sync {
        let source = stream
            .map_err(WebmetroError::from_err)
            .parse_ebml().with_soft_limit(BUFFER_LIMIT)
            .chunk_webm().with_soft_limit(BUFFER_LIMIT);
        let sink = Transmitter::new(self.get_channel());

        source.forward(sink.sink_map_err(|err| -> WebmetroError {match err {}}))
        .into_stream()
        .map(|_| empty())
        .map_err(|err| {
            println!("[Warning] {}", err);
            err
        })
        .flatten()
    }
}

type BoxedBodyStream = Box<Stream<Item = Chunk, Error = WebmetroError> + Send + 'static>;

impl Service for RelayServer {
    type ReqBody = Body;
    type ResBody = WebmPayload<BoxedBodyStream>;
    type Error = WebmetroError;
    type Future = FutureResult<Response<WebmPayload<BoxedBodyStream>>, WebmetroError>;

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let (Parts {method, uri, ..}, request_body) = request.into_parts();

        ok(match (method, uri.path()) {
            (Method::HEAD, "/live") => {
                Response::builder()
                    .header(CONTENT_TYPE, "video/webm")
                    .header("X-Accel-Buffering", "no")
                    .header(CACHE_CONTROL, "no-cache, no-store")
                    .body(WebmPayload(Box::new(empty()) as BoxedBodyStream))
                    .unwrap()
            },
            (Method::GET, "/live") => {
                Response::builder()
                    .header(CONTENT_TYPE, "video/webm")
                    .header("X-Accel-Buffering", "no")
                    .header(CACHE_CONTROL, "no-cache, no-store")
                    .body(WebmPayload(Box::new(self.get_stream()) as BoxedBodyStream))
                    .unwrap()
            },
            (Method::POST, "/live") | (Method::PUT, "/live") => {
                println!("[Info] New source on {}", uri.path());
                Response::new(WebmPayload(Box::new(self.post_stream(request_body)) as BoxedBodyStream))
            },
            _ => {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(WebmPayload(Box::new(empty()) as BoxedBodyStream))
                    .unwrap()
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

    rt::run(Server::bind(&addr)
        .serve(move || {
            ok::<_, WebmetroError>(RelayServer(single_channel.clone()))
        })
        .map_err(|err| {
            println!("[Error] {}", err);
        })
    );

    Ok(())
}
