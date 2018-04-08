extern crate futures;
extern crate hyper;
extern crate lab_ebml;

use std::env::args;
use std::net::ToSocketAddrs;

use futures::future::FutureResult;
use futures::stream::repeat;
use futures::stream::Stream;
use lab_ebml::chunk::{Chunk, WebmStream, ChunkingError};
use lab_ebml::fixers::ChunkStream;
use lab_ebml::stream_parser::StreamEbml;
use hyper::{Get, StatusCode};
use hyper::header::ContentType;
use hyper::server::{Http, Request, Response, Service};

const SRC_FILE: &'static [u8] = include_bytes!("../data/test1.webm");

#[derive(Clone)]
struct WebmServer;

type BodyStream<B> = Box<Stream<Item = Chunk<B>, Error = hyper::Error>>;

impl Service for WebmServer {
    type Request = Request;
    type Response = Response<BodyStream<Vec<u8>>>;
    type Error = hyper::Error;
    type Future = FutureResult<Self::Response, hyper::Error>;
    fn call(&self, req: Request) -> Self::Future {
        let response = match (req.method(), req.path()) {
            (&Get, "/loop") => {
                let stream: BodyStream<Vec<u8>> = Box::new(
                    repeat::<&[u8], ()>(SRC_FILE).take(5)
                    .parse_ebml().chunk_webm().fix_timecodes().find_starting_point()
                    .map_err(|err| match err {
                        ChunkingError::IoError(io_err) => hyper::Error::Io(io_err),
                        ChunkingError::OtherError(_) => hyper::Error::Incomplete
                    })
                );
                Response::new()
                    .with_header(ContentType("video/webm".parse().unwrap()))
                    .with_body(stream)
            },
            _ => {
                Response::new()
                    .with_status(StatusCode::NotFound)
            }
        };
        futures::future::ok(response)
    }
}

pub fn main() {
    let addr = args().nth(1).expect("Need binding address+port").to_socket_addrs().unwrap().next().unwrap();
    Http::new().bind(&addr, move || Ok(WebmServer)).unwrap().run().unwrap();
}
