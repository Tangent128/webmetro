extern crate futures;
extern crate hyper;
extern crate lab_ebml;

use futures::future::FutureResult;
use futures::stream::{iter, Stream};
use lab_ebml::chunk::{Chunk, WebmStream, ChunkingError};
use lab_ebml::Schema;
use lab_ebml::timecode_fixer::ChunkStream;
use lab_ebml::webm::*;
use hyper::{Get, StatusCode};
use hyper::header::ContentType;
use hyper::server::{Http, Request, Response, Service};
use std::env::args;
use std::net::ToSocketAddrs;

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
                let stream: BodyStream<Vec<u8>> = iter(Webm.parse(SRC_FILE).into_iter().map(|x| Ok(x)))
                    .chunk_webm()
                    .chain(iter(Webm.parse(SRC_FILE).into_iter().map(|x| Ok(x))).chunk_webm())
                    .fix_timecodes()
                    .map_err(|err| match err {
                        ChunkingError::IoError(io_err) => hyper::Error::Io(io_err),
                        ChunkingError::OtherError(otx_err) => otx_err
                    })
                    .boxed();
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
    let addr = args().nth(1).unwrap().to_socket_addrs().unwrap().next().unwrap();
    Http::new().bind(&addr, move || Ok(WebmServer)).unwrap().run().unwrap();
}
