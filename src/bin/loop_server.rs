extern crate futures;
extern crate hyper;
extern crate lab_ebml;

use futures::future::FutureResult;
use futures::stream::{iter, Stream};
use hyper::{Get, StatusCode};
use hyper::server::{Http, Request, Response, Service};
use std::env::args;
use std::net::ToSocketAddrs;

//const SRC_FILE: &'static [u8] = include_bytes!("../data/test1.webm");

struct WebmServer;

type BodyStream = Box<Stream<Item = &'static str, Error = hyper::Error>>;

impl Service for WebmServer {
    type Request = Request;
    type Response = Response<BodyStream>;
    type Error = hyper::Error;
    type Future = FutureResult<Self::Response, hyper::Error>;
    fn call(&self, req: Request) -> Self::Future {
        let response = match (req.method(), req.path()) {
            (&Get, "/loop") => {
                let pieces = vec!["<", "Insert WebM stream here.", ">"];
                let stream: BodyStream = iter(pieces.into_iter().map(|x| Ok(x))).boxed();
                Response::new()
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
    Http::new().bind(&addr, || Ok(WebmServer)).unwrap().run().unwrap();
}
