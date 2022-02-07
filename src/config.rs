use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{fs, path::Path};

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

    pub fn read_or_save_default<P: AsRef<Path>>(path: P) -> Result<(Arc<Config>, bool), Error> {
        if !path.as_ref().exists() {
            let config = Config::default();
            let config_json = serde_json::to_string_pretty(&config).unwrap();
            fs::write(&path, config_json)?;
            Ok((Arc::new(config), false))
        } else {
            let config_json = fs::read_to_string(&path)?;
            let config = serde_json::from_str::<Config>(&config_json)?;
            Ok((Arc::new(config), true))
        }
    }
}
