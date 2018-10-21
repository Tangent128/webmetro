use std::error::Error;
use std::net::ToSocketAddrs;
use std::sync::{
    Arc,
    Mutex
};

use bytes::{Bytes, Buf};
use clap::{App, Arg, ArgMatches, SubCommand};
use futures::{
    Future,
    Stream,
    Sink,
    stream::empty
};
use hyper::{
    Body,
    Response,
    header::{
        CACHE_CONTROL,
        CONTENT_TYPE
    }
};
use warp::{
    self,
    Filter
};
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

    fn post_stream(&self, stream: impl Stream<Item = impl Buf, Error = impl Error + Send + Sync + 'static>) -> impl Stream<Item = Bytes, Error = WebmetroError> {
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

    let relay_server = path!("live").map(move || RelayServer(single_channel.clone()));

    let head = relay_server.clone().and(warp::head())
        .map(|_| media_response(Body::empty()));

    let get = relay_server.clone().and(warp::get2())
        .map(|server: RelayServer| media_response(Body::wrap_stream(server.get_stream())));

    let post_put = relay_server.clone().and(warp::post2().or(warp::put2()).unify())
    .and(warp::body::stream()).map(|server: RelayServer, stream| {
        println!("[Info] Source Connected");
        Response::new(Body::wrap_stream(server.post_stream(stream)))
    });

    let routes = head
        .or(get)
        .or(post_put);

    Ok(warp::serve(routes).run(addr))
}
