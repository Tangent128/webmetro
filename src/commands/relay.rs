use std::net::ToSocketAddrs;
use std::sync::{
    Arc,
    Mutex
};

use bytes::Bytes;
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
    Server,
    service::Service,
    header::{
        CACHE_CONTROL,
        CONTENT_TYPE
    }
};
use tokio::runtime::Runtime;
use webmetro::{
    channel::{
        Channel,
        Listener,
        Transmitter
    },
    chunk::WebmStream,
    error::WebmetroError,
    fixers::ChunkStream,
    stream_parser::StreamEbml
};

const BUFFER_LIMIT: usize = 2 * 1024 * 1024;

struct RelayServer(Arc<Mutex<Channel>>);

impl RelayServer {
    fn get_channel(&self) -> Arc<Mutex<Channel>> {
        self.0.clone()
    }

    fn get_stream(&self) -> impl Stream<Item = Bytes, Error = WebmetroError> {
        Listener::new(self.get_channel())
        .fix_timecodes()
        .find_starting_point()
        .map(|webm_chunk| webm_chunk.into_bytes())
        .map_err(|err| match err {})
    }

    fn post_stream(&self, stream: Body) -> impl Stream<Item = Bytes, Error = WebmetroError> {
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

fn media_response(body: Body) -> Response<Body> {
    Response::builder()
        .header(CONTENT_TYPE, "video/webm")
        .header("X-Accel-Buffering", "no")
        .header(CACHE_CONTROL, "no-cache, no-store")
        .body(body)
        .unwrap()
}

impl Service for RelayServer {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = WebmetroError;
    type Future = FutureResult<Response<Body>, WebmetroError>;

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let (Parts {method, uri, ..}, request_body) = request.into_parts();

        ok(match (method, uri.path()) {
            (Method::HEAD, "/live") => media_response(Body::empty()),
            (Method::GET, "/live") => media_response(Body::wrap_stream(self.get_stream())),
            (Method::POST, "/live") | (Method::PUT, "/live") => {
                println!("[Info] New source on {}", uri.path());
                Response::new(Body::wrap_stream(self.post_stream(request_body)))
            },
            _ => {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
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

    Runtime::new().unwrap().block_on_all(Server::bind(&addr)
        .serve(move || {
            ok::<_, WebmetroError>(RelayServer(single_channel.clone()))
        }).map_err(|err| WebmetroError::Unknown(Box::new(err)))
    )
}
