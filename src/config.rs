use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::{fs, io::ErrorKind, path::Path};

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    driver: String,
    song_format: String,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            driver: "spotify-desktop".into(),
            song_format: "♫ {artist} - {title}".into(),
        }
    }
}

impl Config {
    pub fn driver_name(&self) -> &str {
        self.driver.as_str()
    }

    pub fn song_format(&self) -> &str {
        self.song_format.as_str()
    }

    /// Attempts to read and deserialize a new [Config] instance
    /// from a file with the provided path.
    pub fn try_read<P>(path: P) -> Result<Config, Error>
    where
        P: AsRef<Path>,
    {
        match fs::read_to_string(&path) {
            Ok(config_json) => {
                let config = serde_json::from_str::<Config>(&config_json)?;
                Ok(config)
            }
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    println!("The config file was not found.");
                } else {
                    println!("The config file could not be read.");
                }
                Err(err.into())
            }
        }
    }

    /// Attempts to serialize this [Config] instance to a file.
    pub fn try_save<P>(&self, path: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        let config_json = serde_json::to_string_pretty(&self)?;
        fs::write(&path, config_json)?;
        Ok(())
    }
}
