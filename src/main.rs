#[macro_use]
extern crate clap;
extern crate streamlink;

use clap::{App, Arg, SubCommand};
use std::env;
use std::path::Path;
use std::process;
use streamlink::run;

fn main() {
    let matches = App::new("strs")
        .about("streamlink interface")
        .version(crate_version!())
        .subcommand(SubCommand::with_name("list").about("list streamers"))
        .subcommand(SubCommand::with_name("url").about("print formatted URL"))
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .takes_value(true)
                .value_name("FILE"),
        )
        .get_matches();

    let default_config_path = match env::home_dir() {
        Some(pb) => pb.join(".config/streamlink-rs/config.toml"),
        None => {
            println!("failed to get home directory");
            process::exit(2);
        }
    };
    let config_path: &Path = match matches.value_of("config") {
        Some(path) => Path::new(path),
        None => default_config_path.as_path(),
    };
    run(config_path);
}
