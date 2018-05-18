use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use toml;

use errors::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub stream_urls: Vec<String>,
}

impl Config {
    pub fn new<P>(filepath: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut config = String::new();
        let mut f = File::open(filepath)?;
        f.read_to_string(&mut config).unwrap();
        let config: Config = toml::from_str(config.as_str()).unwrap();
        Ok(config)
    }
}
