extern crate bytes;
#[macro_use] extern crate clap;
extern crate futures;
extern crate http;
extern crate hyper;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_io;
#[macro_use] extern crate warp;
extern crate weak_table;
extern crate webmetro;

mod commands;

use clap::{App, AppSettings};

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
        ("filter", Some(sub_args)) => filter::run(sub_args),
        ("relay", Some(sub_args)) => relay::run(sub_args),
        ("send", Some(sub_args)) => send::run(sub_args),
        ("dump", Some(sub_args)) => dump::run(sub_args),
        _ => {
            options().print_help().unwrap();
            println!("");
            Ok(())
        }
    }.unwrap_or_else(|err| {
        eprintln!("Error: {}", err);
    });
}
