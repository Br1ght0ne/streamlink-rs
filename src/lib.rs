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

mod config;

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

#[derive(Debug, PartialEq, Eq)]
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

    pub fn name(&self) -> &str {
        self.url.path().split('/').next().unwrap()
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
    urls: Vec<Stream>,
}

impl Streamlink {
    pub fn new(config: Config) -> Result<Self, UrlError> {
        Ok(Self::from_strings(config.stream_urls)?)
    }

    pub fn from_strs(strs: &[&str]) -> Result<Self, UrlError> {
        let urls: Vec<Url> = strs
            .into_iter()
            .map(|s| Url::parse(s).map_err(|_| UrlError::Malformed))
            .map(|s| s.or_else(Err).unwrap())
            .collect();
        Ok(Self::from_urls(urls)?)
    }

    pub fn from_strings(strings: Vec<String>) -> Result<Self, UrlError> {
        let strs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
        Self::from_strs(&strs)
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
        .map(|(url, status)| {
            progress_bar.inc(1);
            format!(
                "{} is {}",
                url,
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
        pub const RIGHT_URL_STR: &str = "https://twitch.tv/gogcom";
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
            assert_eq!(UrlKind::Twitch, kind(constants::RIGHT_URL_STR));
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

        pub fn stream(s: &str) -> Stream {
            Stream::from_str(s).expect("wrong str")
        }

        #[test]
        fn from_right_url_str() {
            // `Stream` can be created from a correct URL str.
            stream(constants::RIGHT_URL_STR);
        }

        #[test]
        fn from_wrong_url_str() {
            // `Stream` can NOT be created from an incorrect URL str.
            Stream::from_str(constants::WRONG_URL_STR).expect_err("right str");
            Stream::from_str(&constants::RIGHT_URL_STR.replace("https://", ""))
                .expect_err("right str");
        }
    }

    mod status {
        use super::constants;
        use super::stream::stream;
        use *;

        pub fn status(s: &str) -> StreamStatus {
            stream(s).status().expect("failed to get status")
        }

        #[test]
        fn can_get() {
            // `Stream.status()` works for valid URL strs.
            status(constants::RIGHT_URL_STR);
        }

        #[test]
        fn always_offline() {
            assert_eq!(StreamStatus::Offline, status(constants::ALWAYS_OFF_URL_STR));
        }

        #[test]
        fn always_online() {
            assert_eq!(StreamStatus::Online, status(constants::ALWAYS_ON_URL_STR));
        }
    }
}
