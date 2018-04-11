#[macro_use]
extern crate clap;

use clap::{App, AppSettings};

fn main() {
    let args = App::new("webmetro")
        .version(crate_version!())
        .about("Utilities for broadcasting & relaying live WebM video/audio streams")
        .setting(AppSettings::SubcommandRequired)
        .setting(AppSettings::VersionlessSubcommands)
        .get_matches();

    match args.subcommand() {
        _ => {}
    }
}
