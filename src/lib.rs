#![feature(conservative_impl_trait)]
extern crate console;
extern crate indicatif;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate url;

use console::style;
use indicatif::ProgressBar;
use std::fmt;
use std::io;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::str::FromStr;
use url::{Host, ParseError, Url};

pub mod config;

use config::Config;

#[derive(Debug, PartialEq, Eq)]
enum UrlKind {
    Youtube,
    Twitch,
    Other,
}

#[derive(Debug, PartialEq)]
pub enum StreamStatus {
    Online,
    Offline,
}

impl fmt::Display for StreamStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let s: &'static str = match self {
            StreamStatus::Offline => "offline",
            StreamStatus::Online => "online",
        };
        write!(f, "{}", s)
    }
}

impl<'a> From<&'a Url> for UrlKind {
    fn from(url: &Url) -> Self {
        match url.host() {
            Some(Host::Domain(host)) => match host {
                "youtube.com" => UrlKind::Youtube,
                "twitch.tv" => UrlKind::Twitch,
                _ => UrlKind::Other,
            },
            _ => UrlKind::Other,
        }
    }
}

/// Represents a stream of a specific `kind` on a specific `url`.
#[derive(Debug, PartialEq)]
pub struct Stream {
    url: Url,
    kind: UrlKind,
}

#[derive(Debug, PartialEq)]
pub enum UrlError {
    NonStream,
    Malformed,
}

impl From<ParseError> for UrlError {
    fn from(_e: ParseError) -> Self {
        UrlError::Malformed
    }
}

impl Stream {
    pub fn from_url(url: Url) -> Result<Self, UrlError> {
        let kind = UrlKind::from(&url);
        match kind {
            UrlKind::Other => Err(UrlError::NonStream),
            _ => Ok(Self { url, kind }),
        }
    }

    /// Returns the name (aka ID) of the stream.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use streamlink::Stream;
    /// use std::str::FromStr;
    ///
    /// let stream = Stream::from_str("https://twitch.tv/gogcom").unwrap();
    /// assert_eq!("gogcom", stream.name().unwrap());
    ///
    /// let stream = Stream::from_str("https://youtube.com/user/markiplierGAME").unwrap();
    /// assert_eq!("markiplierGAME", stream.name().unwrap());
    /// ```
    pub fn name(&self) -> Option<&str> {
        let path = self.url.path();
        let mut path_parts = path.split('/').skip(1);

        match self.kind {
            UrlKind::Twitch => path_parts.next(),
            UrlKind::Youtube => match path_parts.next() {
                Some("user") => path_parts.next(),
                Some(id) => Some(id),
                None => None,
            },
            UrlKind::Other => None,
        }
    }

    // TODO: proper implementation
    /// Checks if stream is online.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use streamlink::{Stream, StreamStatus};
    /// use std::str::FromStr;
    ///
    /// let online_stream_url = Stream::from_str("https://twitch.tv/food").unwrap();
    /// assert_eq!(StreamStatus::Online, online_stream_url.status().unwrap());
    ///
    /// let offline_stream_url = Stream::from_str("https://twitch.tv/some_offline_stream").unwrap();
    /// assert_eq!(StreamStatus::Offline, offline_stream_url.status().unwrap());
    /// ```
    ///
    /// # Errors
    ///
    /// If `youtube-dl` failed to execute, [`std::io::Error`] will be returned.
    pub fn status(&self) -> Result<StreamStatus, io::Error> {
        let status: ExitStatus = Command::new("youtube-dl")
            .args(&["-F", self.url.as_str()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        let status = if status.success() {
            StreamStatus::Online
        } else {
            StreamStatus::Offline
        };
        Ok(status)
    }
}

impl FromStr for Stream {
    type Err = UrlError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Url::parse(s) {
            Ok(url) => Stream::from_url(url),
            Err(_) => Err(UrlError::Malformed),
        }
    }
}

impl fmt::Display for Stream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.url)
    }
}

#[derive(Debug)]
pub struct Streamlink {
    pub urls: Vec<Stream>,
}

impl Streamlink {
    pub fn new(config: Config) -> Result<Self, UrlError> {
        Ok(Self::from_strings(config.stream_urls)?)
    }

    pub fn from_strs(strs: Vec<&str>) -> Result<Self, UrlError> {
        Self::from_strings(strs.into_iter().map(String::from).collect())
    }

    pub fn from_strings(strings: Vec<String>) -> Result<Self, UrlError> {
        let urls: Vec<Url> = strings
            .into_iter()
            .map(|s| Url::parse(s.as_str()).map_err(|_| UrlError::Malformed))
            .map(|s| s.or_else(Err).unwrap())
            .collect();
        Ok(Self::from_urls(urls)?)
    }

    pub fn from_urls(urls: Vec<Url>) -> Result<Self, UrlError> {
        let urls: Vec<Stream> = urls
            .into_iter()
            .map(|u| Stream::from_url(u).or_else(Err).unwrap())
            .collect();
        Ok(Self { urls })
    }

    pub fn status(&self) -> impl Iterator<Item = (&Stream, StreamStatus)> {
        let urls_iter = self.urls.iter();
        let statuses_iter = self
            .urls
            .iter()
            .map(|url| url.status().unwrap_or(StreamStatus::Offline));
        urls_iter.zip(statuses_iter)
    }

    pub fn stream_urls(&self) -> &Vec<Stream> {
        &self.urls
    }
}

pub fn run<P: AsRef<Path>>(config_path: P) {
    let config = Config::new(config_path).expect("error while reading config");
    let progress_bar = ProgressBar::new(config.stream_urls.len() as u64);
    let streamlink = Streamlink::new(config).unwrap();
    let status = streamlink.status();
    let lines: Vec<String> = status
        .map(|(stream, status)| {
            progress_bar.inc(1);
            format!(
                "{} is {}",
                stream.name().unwrap_or_else(|| stream.url.as_str()),
                match status {
                    StreamStatus::Offline => style(status).red(),
                    StreamStatus::Online => style(status).green(),
                }
            )
        })
        .collect();
    progress_bar.finish_and_clear();
    for line in lines {
        println!("{}", line);
    }
}

#[cfg(test)]
mod tests {

    mod constants {
        pub const TWITCH_GOGCOM: &str = "https://twitch.tv/gogcom";
        pub const YOUTUBE_MARKIPLIERGAME_USER: &str = "https://youtube.com/user/markiplierGAME";
        pub const YOUTUBE_MARKIPLIERGAME_DIRECT: &str = "https://youtube.com/markiplierGAME";
        pub const OTHER_VALID: &str = "https://rust-lang.org/about";
        pub const ALWAYS_OFF_URL_STR: &str = "https://twitch.tv/NotRealBrightOneLOL";
        pub const ALWAYS_ON_URL_STR: &str = "https://twitch.tv/food";
        pub const WRONG_URL_STR: &str = "wrong://fake.tv/thisdefinitelydoesntexist";
    }

    mod url_kind {
        use super::constants;
        use *;

        fn kind(s: &str) -> UrlKind {
            UrlKind::from(&Url::parse(s).unwrap())
        }

        #[test]
        fn youtube() {
            assert_eq!(UrlKind::Youtube, kind("https://youtube.com/markipliergame"));
        }

        #[test]
        fn twitch() {
            assert_eq!(UrlKind::Twitch, kind(constants::TWITCH_GOGCOM));
        }

        #[test]
        fn other() {
            assert_eq!(UrlKind::Other, kind("https://rust-lang.org"));
        }

        #[test]
        #[should_panic]
        fn malformed() {
            kind("this is not an URL");
        }
    }

    mod stream {
        use super::constants;
        use *;

        pub fn stream_from_str(s: &str) -> Stream {
            Stream::from_str(s).expect("wrong str")
        }

        #[test]
        fn from_right_url_str() {
            // `Stream` can be created from a correct URL str.
            stream_from_str(constants::TWITCH_GOGCOM);
        }

        #[test]
        fn from_wrong_url_str() {
            // `Stream` can NOT be created from an incorrect URL str.
            Stream::from_str(constants::WRONG_URL_STR).expect_err("right str");
            Stream::from_str(&constants::TWITCH_GOGCOM.replace("https://", ""))
                .expect_err("right str");
        }

        mod name {
            use super::*;

            #[test]
            fn twitch() {
                assert_eq!(
                    "gogcom",
                    stream_from_str(constants::TWITCH_GOGCOM).name().unwrap()
                );
            }

            #[test]
            fn youtube_user() {
                assert_eq!(
                    "markiplierGAME",
                    stream_from_str(constants::YOUTUBE_MARKIPLIERGAME_USER)
                        .name()
                        .unwrap()
                );
            }

            #[test]
            fn youtube_direct() {
                assert_eq!(
                    "markiplierGAME",
                    stream_from_str(constants::YOUTUBE_MARKIPLIERGAME_DIRECT)
                        .name()
                        .unwrap()
                );
            }

            #[test]
            #[should_panic]
            fn other() {
                stream_from_str(constants::OTHER_VALID).name();
            }
        }
    }

    mod status {
        use super::constants;
        use super::stream::stream_from_str;
        use *;

        pub fn status_from_str(s: &str) -> StreamStatus {
            stream_from_str(s).status().expect("failed to get status")
        }

        #[test]
        fn can_get() {
            // `Stream.status()` works for valid URL strs.
            status_from_str(constants::TWITCH_GOGCOM);
        }

        #[test]
        fn always_offline() {
            assert_eq!(
                StreamStatus::Offline,
                status_from_str(constants::ALWAYS_OFF_URL_STR)
            );
        }

        #[test]
        fn always_online() {
            assert_eq!(
                StreamStatus::Online,
                status_from_str(constants::ALWAYS_ON_URL_STR)
            );
        }
    }
}
