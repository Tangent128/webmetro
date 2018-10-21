#[macro_use] extern crate clap;
extern crate futures;
extern crate http;
extern crate hyper;
extern crate tokio_codec;
extern crate tokio_io;
extern crate webmetro;

mod commands;

use clap::{App, AppSettings};
use futures::prelude::*;
use hyper::rt;
use webmetro::error::WebmetroError;

use commands::{
    relay,
    filter,
    send,
    dump
};

fn options() -> App<'static, 'static> {
    App::new("webmetro")
        .version(crate_version!())
        .about("Utilities for broadcasting & relaying live WebM video/audio streams")
        .setting(AppSettings::DisableHelpSubcommand)
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(relay::options())
        .subcommand(filter::options())
        .subcommand(send::options())
        .subcommand(dump::options())
}

fn main() {
    let args = options().get_matches();

    tokio_run(match args.subcommand() {
        ("filter", Some(sub_args)) => box_up(filter::run(sub_args)),
        ("relay", Some(sub_args)) => box_up(relay::run(sub_args)),
        ("send", Some(sub_args)) => box_up(send::run(sub_args)),
        ("dump", Some(sub_args)) => box_up(dump::run(sub_args)),
        _ => box_up(futures::lazy(|| {
            options().print_help().unwrap();
            println!("");
            Ok(())
        }))
    });
}

fn tokio_run(task: Box<Future<Item=(), Error=WebmetroError> + Send>) {
    rt::run(task.into_future().map_err(|err| {
        eprintln!("Error: {}", err);
        ::std::process::exit(1);
    }));
}

fn box_up<F: IntoFuture<Item=(), Error=WebmetroError>>(task: F) -> Box<Future<Item=(), Error=WebmetroError> + Send>
where F::Future: Send + 'static
{
    Box::new(task.into_future())
}
