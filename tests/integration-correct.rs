extern crate streamlink;

use streamlink::{Config, Stream, Streamlink};

fn main() {
    let streamlink =
        Streamlink::new(Config::new("config.toml").unwrap()).expect("error while parsing URL");

    assert_eq!(
        vec![Stream::from_string("https://twitch.tv/food".into()).unwrap()],
        streamlink.urls
    );
}
