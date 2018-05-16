#[macro_use]
extern crate clap;
extern crate streamlink;

use clap::{App, Arg, SubCommand};
use std::env;
use std::path::{Path, PathBuf};
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

    let home_dir = env::home_dir().expect("failed to get home dir");
    let default_config_path_buf: PathBuf = home_dir.join(".config/streamlink-rs/config.toml");
    let default_config_path: &Path = default_config_path_buf.as_path();
    let config_path: &Path = match matches.value_of("config") {
        Some(path) => Path::new(path),
        None => default_config_path,
    };
    run(config_path);
}
