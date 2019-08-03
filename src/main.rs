#[macro_use]
extern crate clap;
extern crate dirs;
extern crate streamlink;

use clap::{App, Arg, SubCommand};
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

    let default_config_path = dirs::home_dir()
        .unwrap_or(PathBuf::new().join("/"))
        .join(".config/streamlink-rs/config.toml");
    let config_path: &Path = match matches.value_of("config") {
        Some(path) => Path::new(path),
        None => default_config_path.as_path(),
    };
    if let Err(ref e) = run(config_path) {
        println!("error: {}", e);

        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }

        ::std::process::exit(1);
    }
}
