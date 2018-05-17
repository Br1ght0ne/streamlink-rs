extern crate streamlink;

use std::str::FromStr;
use streamlink::{config::Config, Stream, Streamlink};

fn main() {
    let streamlink =
        Streamlink::new(Config::new("config.toml").unwrap()).expect("error while parsing URL");

    assert_eq!(
        vec![Stream::from_str("https://twitch.tv/food").unwrap()],
        streamlink.urls
    );
}
