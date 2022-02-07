use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Config {
    song_format: String,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            song_format: "♫ {artist} - {title}".into(),
        }
    }
}

impl Config {
    pub fn song_format(&self) -> &str {
        self.song_format.as_str()
    }
}
