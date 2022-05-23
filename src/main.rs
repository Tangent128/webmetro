#[macro_use]
extern crate log;

mod commands;

use clap::{Parser, Subcommand};

/// Utilities for broadcasting & relaying live WebM video/audio streams
#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Dump(commands::dump::DumpArgs),
    Filter(commands::filter::FilterArgs),
    Relay(commands::relay::RelayArgs),
    Send(commands::send::SendArgs),
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    match args.command {
        Command::Dump(args) => commands::dump::run(args),
        Command::Filter(args) => commands::filter::run(args),
        Command::Relay(args) => commands::relay::run(args),
        Command::Send(args) => commands::send::run(args),
    }
    .unwrap_or_else(|err| {
        error!("{}", err);
    });
}
