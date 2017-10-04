extern crate futures;
extern crate hyper;
extern crate lab_ebml;

use futures::future::FutureResult;
use futures::stream::{once, iter, Stream};
use lab_ebml::chunk::Chunk;
use lab_ebml::Schema;
use lab_ebml::timecode_fixer::ChunkStream;
use lab_ebml::webm::*;
use lab_ebml::webm::WebmElement::*;
use hyper::{Get, StatusCode};
use hyper::header::ContentType;
use hyper::server::{Http, Request, Response, Service};
use std::env::args;
use std::io::Cursor;
use std::net::ToSocketAddrs;
use std::sync::Arc;

const SRC_FILE: &'static [u8] = include_bytes!("../data/test1.webm");

#[derive(Clone)]
struct WebmServer(Chunk, Vec<Chunk>);

type BodyStream<B> = Box<Stream<Item = Chunk<B>, Error = hyper::Error>>;

impl Service for WebmServer {
    type Request = Request;
    type Response = Response<BodyStream<Vec<u8>>>;
    type Error = hyper::Error;
    type Future = FutureResult<Self::Response, hyper::Error>;
    fn call(&self, req: Request) -> Self::Future {
        let response = match (req.method(), req.path()) {
            (&Get, "/loop") => {
                let results: Vec<Result<Chunk, ()>> = self.1.iter().map(|x| Ok(x.clone())).collect();
                let stream: BodyStream<Vec<u8>> = Box::new(
                    once(Ok(self.0.clone()))
                    .chain(iter(results.into_iter().cycle().take(20)))
                    .map_err(|_| hyper::Error::Incomplete)
                    .fix_timecodes()
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
    let addr = args().nth(1).unwrap().to_socket_addrs().unwrap().next().unwrap();
    let webm_service = create_loop();
    Http::new().bind(&addr, move || Ok(webm_service.clone())).unwrap().run().unwrap();
}

fn create_loop() -> WebmServer {
    let mut header = None;
    let mut reading_head = true;

    let mut cluster_header = None;
    let mut cluster_timecode = 0;
    let mut chunks = Vec::new();

    let mut buffer = Cursor::new(Vec::new());

    for element in Webm.parse(SRC_FILE) {
        match element {
            Cluster => {
                if reading_head {
                    header = Some(Chunk::Headers {bytes: Arc::new(buffer.into_inner())});
                } else {
                    if let Some(chunk) = cluster_header.take() {
                        chunks.push(chunk);
                    }
                    chunks.push(Chunk::ClusterBody {bytes: Arc::new(buffer.into_inner())});
                }
                buffer = Cursor::new(Vec::new());
                reading_head = false;
            },
            Timecode(timecode) => {
                cluster_timecode = timecode;
                cluster_header = Some(Chunk::<Vec<u8>>::new_cluster_head(timecode));
            },
            SimpleBlock(ref block) => {
                if let Some(ref mut chunk) = cluster_header {
                    if (block.flags & 0b10000000) != 0 {
                        // TODO: this is incorrect, condition needs to also affirm we're the first video block of the cluster
                        chunk.mark_keyframe(true);
                    }
                    chunk.observe_simpleblock_timecode(block.timecode);
                }
                encode_webm_element(&SimpleBlock(*block), &mut buffer).unwrap();
            },
            Info => continue,
            Void => continue,
            Unknown(_) => continue,
            ref other => {
                encode_webm_element(other, &mut buffer).unwrap();
            },
        }
    }

    // finish last cluster
    if let Some(chunk) = cluster_header.take() {
        chunks.push(chunk);
    }
    chunks.push(Chunk::ClusterBody {bytes: Arc::new(buffer.into_inner())});

    WebmServer(header.unwrap(), chunks)
}
