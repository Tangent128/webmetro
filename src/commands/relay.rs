use std::net::ToSocketAddrs;
use std::sync::{
    Arc,
    Mutex,
    Weak
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
    Filter,
    path
};
use weak_table::{
    WeakValueHashMap
};
use webmetro::{
    channel::{
        Channel,
        Handle,
        Listener,
        Transmitter
    },
    chunk::WebmStream,
    error::WebmetroError,
    fixers::ChunkStream,
    stream_parser::StreamEbml
};

const BUFFER_LIMIT: usize = 2 * 1024 * 1024;

fn get_stream(channel: Handle) -> impl Stream<Item = Bytes, Error = WebmetroError> {
    Listener::new(channel)
    .fix_timecodes()
    .find_starting_point()
    .map(|webm_chunk| webm_chunk.into_bytes())
    .map_err(|err| match err {})
}

fn post_stream(channel: Handle, stream: impl Stream<Item = impl Buf, Error = warp::Error>) -> impl Stream<Item = Bytes, Error = WebmetroError> {
    let source = stream
        .map_err(WebmetroError::from)
        .parse_ebml().with_soft_limit(BUFFER_LIMIT)
        .chunk_webm().with_soft_limit(BUFFER_LIMIT);
    let sink = Transmitter::new(channel);

    source.forward(sink.sink_map_err(|err| -> WebmetroError {match err {}}))
    .into_stream()
    .map(|_| empty())
    .map_err(|err| {
        println!("[Warning] {}", err);
        err
    })
    .flatten()
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
    let channel_map = Arc::new(Mutex::new(WeakValueHashMap::<String, Weak<Mutex<Channel>>>::new()));

    let addr_str = args.value_of("listen").ok_or("Listen address wasn't provided")?;
    let addr = addr_str.to_socket_addrs()?.next().ok_or("Listen address didn't resolve")?;

    let channel = path!("live" / String).map(move |name: String| {
        let channel = channel_map.lock().unwrap()
            .entry(name.clone())
            .or_insert_with(|| Channel::new(name.clone()));
        (channel, name)
    });

    let head = channel.clone().and(warp::head())
        .map(|(_, name)| {
            println!("[Info] HEAD Request For Channel {}", name);
            media_response(Body::empty())
        });

    let get = channel.clone().and(warp::get2())
        .map(|(channel, name)| {
            println!("[Info] Listener Connected On Channel {}", name);
            media_response(Body::wrap_stream(get_stream(channel)))
        });

    let post_put = channel.clone().and(warp::post2().or(warp::put2()).unify())
        .and(warp::body::stream()).map(|(channel, name), stream| {
            println!("[Info] Source Connected On Channel {}", name);
            Response::new(Body::wrap_stream(post_stream(channel, stream)))
        });

    let routes = head
        .or(get)
        .or(post_put);

    Ok(warp::serve(routes).run(addr))
}
