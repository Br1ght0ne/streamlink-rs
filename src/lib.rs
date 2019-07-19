#![recursion_limit = "1024"]
extern crate ansi_term;
#[macro_use]
extern crate error_chain;
extern crate indicatif;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate url;

use ansi_term::Colour::{Green, Red};
use indicatif::ProgressBar;
use std::fmt;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use url::{Host, Url};

mod config;

pub use config::Config;

mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
        }

        errors {
            NonStreamUrl(url: String) {
                description("non-stream URL")
                display("non-stream URL: '{}'", url)
            }
            UrlParse(url: String) {
                description("failed to parse URL")
                display("failed to parse URL: '{}'", url)
            }
        }
    }
}

use errors::*;

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
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
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

impl Stream {
    pub fn from_url(url: Url) -> Result<Self> {
        let kind = UrlKind::from(&url);
        match kind {
            UrlKind::Other => bail!(ErrorKind::NonStreamUrl(url.as_str().into())),
            _ => Ok(Self { url, kind }),
        }
    }

    pub fn from_string(s: String) -> Result<Self> {
        let url: Url = Url::parse(s.as_str()).chain_err(|| ErrorKind::UrlParse(s))?;
        Ok(Self::from_url(url)?)
    }
    /// Returns the name (aka ID) of the stream.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use streamlink::Stream;
    /// use std::str::FromStr;
    ///
    /// let stream = Stream::from_string("https://twitch.tv/gogcom".into()).unwrap();
    /// assert_eq!("gogcom", stream.name().unwrap());
    ///
    /// let stream = Stream::from_string("https://youtube.com/user/markiplierGAME".into()).unwrap();
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
    ///
    /// let online_stream_url = Stream::from_string("https://twitch.tv/food".into()).unwrap();
    /// assert_eq!(StreamStatus::Online, online_stream_url.status().unwrap());
    ///
    /// let offline_stream_url = Stream::from_string("https://twitch.tv/some_offline_stream".into()).unwrap();
    /// assert_eq!(StreamStatus::Offline, offline_stream_url.status().unwrap());
    /// ```
    ///
    /// # Errors
    ///
    /// If `youtube-dl` failed to execute, [`std::io::Error`] will be returned.
    pub fn status(&self) -> Result<StreamStatus> {
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
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self::from_strings(config.stream_urls)?)
    }

    pub fn from_strs(strs: Vec<&str>) -> Result<Self> {
        Self::from_strings(strs.into_iter().map(String::from).collect())
    }

    pub fn from_strings(strings: Vec<String>) -> Result<Self> {
        let mut urls: Vec<Url> = vec![];
        for string in strings {
            let url = Url::parse(string.as_str());
            match url {
                Ok(url) => urls.push(url),
                Err(_) => bail!(ErrorKind::UrlParse(string)),
            }
        }
        Ok(Self::from_urls(urls).chain_err(|| "failed to create from urls")?)
    }

    pub fn from_urls(urls: Vec<Url>) -> Result<Self> {
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

pub fn run<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config = Config::new(config_path).chain_err(|| "unable to create config")?;
    let progress_bar = ProgressBar::new(config.stream_urls.len() as u64);
    let streamlink = Streamlink::new(config).chain_err(|| "unable to create streamlink")?;
    let status = streamlink.status();
    let lines: Vec<String> = status
        .map(|(stream, status)| {
            progress_bar.inc(1);
            format!(
                "{} is {}",
                stream.name().unwrap_or_else(|| stream.url.as_str()),
                match status {
                    StreamStatus::Offline => Red.paint(format!("{}", status)),
                    StreamStatus::Online => Green.paint(format!("{}", status)),
                }
            )
        })
        .collect();
    progress_bar.finish_and_clear();
    for line in lines {
        println!("{}", line);
    }
    Ok(())
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

        fn kind(s: String) -> UrlKind {
            UrlKind::from(&Url::parse(s.as_str()).unwrap())
        }

        #[test]
        fn youtube() {
            assert_eq!(
                UrlKind::Youtube,
                kind(constants::YOUTUBE_MARKIPLIERGAME_USER.into())
            );
            assert_eq!(
                UrlKind::Youtube,
                kind(constants::YOUTUBE_MARKIPLIERGAME_DIRECT.into())
            );
        }

        #[test]
        fn twitch() {
            assert_eq!(UrlKind::Twitch, kind(constants::TWITCH_GOGCOM.into()));
        }

        #[test]
        fn other() {
            assert_eq!(UrlKind::Other, kind(constants::OTHER_VALID.into()));
        }

        #[test]
        #[should_panic]
        fn malformed() {
            kind("this is not an URL".into());
        }
    }

    mod stream {
        use super::constants;
        use *;

        pub fn stream_from_string(s: String) -> Stream {
            Stream::from_url(Url::parse(s.as_str()).unwrap()).expect("wrong str")
        }

        #[test]
        fn from_right_url_str() {
            // `Stream` can be created from a correct URL str.
            stream_from_string(constants::TWITCH_GOGCOM.into());
        }

        #[test]
        #[should_panic]
        fn from_wrong_url_str() {
            // `Stream` can NOT be created from an incorrect URL str.
            stream_from_string(constants::WRONG_URL_STR.into());
            stream_from_string(constants::TWITCH_GOGCOM.replace("https://", ""));
        }

        mod name {
            use super::*;

            #[test]
            fn twitch() {
                assert_eq!(
                    "gogcom",
                    stream_from_string(constants::TWITCH_GOGCOM.into())
                        .name()
                        .unwrap()
                );
            }

            #[test]
            fn youtube_user() {
                assert_eq!(
                    "markiplierGAME",
                    stream_from_string(constants::YOUTUBE_MARKIPLIERGAME_USER.into())
                        .name()
                        .unwrap()
                );
            }

            #[test]
            fn youtube_direct() {
                assert_eq!(
                    "markiplierGAME",
                    stream_from_string(constants::YOUTUBE_MARKIPLIERGAME_DIRECT.into())
                        .name()
                        .unwrap()
                );
            }

            #[test]
            #[should_panic]
            fn other() {
                stream_from_string(constants::OTHER_VALID.into()).name();
            }
        }
    }

    mod status {
        use super::constants;
        use super::stream::stream_from_string;
        use *;

        pub fn status_from_str(s: String) -> StreamStatus {
            stream_from_string(s)
                .status()
                .expect("failed to get status")
        }

        #[test]
        fn can_get() {
            // `Stream.status()` works for valid URL strs.
            status_from_str(constants::TWITCH_GOGCOM.into());
        }

        #[test]
        fn always_offline() {
            assert_eq!(
                StreamStatus::Offline,
                status_from_str(constants::ALWAYS_OFF_URL_STR.into())
            );
        }

        #[test]
        fn always_online() {
            assert_eq!(
                StreamStatus::Online,
                status_from_str(constants::ALWAYS_ON_URL_STR.into())
            );
        }
    }
}
