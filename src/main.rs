
#[macro_use] extern crate log;

mod commands;

use clap::{App, AppSettings, crate_version};

use crate::commands::{
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
    env_logger::init();
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
        error!("{}", err);
    });
}
