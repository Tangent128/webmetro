extern crate futures;
extern crate hyper;
extern crate lab_ebml;

use futures::future::FutureResult;
use hyper::StatusCode;
use hyper::server::{Http, Request, Response, Service};
use std::env::args;
use std::net::ToSocketAddrs;

const SRC_FILE: &'static [u8] = include_bytes!("../data/test1.webm");

struct WebmServer;

impl Service for WebmServer {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = FutureResult<Response, hyper::Error>;
    fn call(&self, req: Request) -> Self::Future {
        futures::future::ok(Response::new().with_status(StatusCode::NotFound))
    }
}

pub fn main() {
    let addr = args().nth(1).unwrap().to_socket_addrs().unwrap().next().unwrap();
    Http::new().bind(&addr, || Ok(WebmServer)).unwrap().run().unwrap();
}
