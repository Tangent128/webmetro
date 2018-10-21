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

    match args.subcommand() {
        ("filter", Some(sub_args)) => { tokio_run(filter::run(sub_args)); },
        ("relay", Some(sub_args)) => { relay::run(sub_args).unwrap_or_else(handle_error); },
        ("send", Some(sub_args)) => { tokio_run(send::run(sub_args)); },
        ("dump", Some(sub_args)) => { dump::run(sub_args).unwrap_or_else(handle_error); },
        _ => {
            options().print_help().unwrap();
            println!("");
        }
    };
}

fn handle_error(err: WebmetroError) {
    eprintln!("Error: {}", err);
}

fn tokio_run<T: IntoFuture<Item=(), Error=WebmetroError> + Send>(task: T)
where T::Future: Send + 'static {
    rt::run(task.into_future().map_err(|err| {
        handle_error(err);
        ::std::process::exit(1);
    }));
}
