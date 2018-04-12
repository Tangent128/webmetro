#[macro_use] extern crate clap;
extern crate futures;
extern crate hyper;
extern crate webmetro;

mod commands;

use clap::{App, AppSettings};
use commands::{
    relay,
    dump
};

fn options() -> App<'static, 'static> {
    App::new("webmetro")
        .version(crate_version!())
        .about("Utilities for broadcasting & relaying live WebM video/audio streams")
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(relay::options())
        .subcommand(dump::options())
}

fn main() {
    let args = options().get_matches();

    match args.subcommand() {
        ("relay", Some(sub_args)) => relay::run(sub_args),
        ("dump", Some(sub_args)) => dump::run(sub_args),
        _ => {
            options().print_help().unwrap();
            println!("");
            Ok(())
        }
    }.unwrap_or_else(|err| println!("Error: {}", err));
}
