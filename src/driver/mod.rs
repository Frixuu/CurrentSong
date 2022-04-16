use crate::song::SongInfo;

use self::spotify_desktop::SpotifyDesktopDriver;

mod noop;
mod spotify_desktop;

pub use noop::noop;

/// Driver connects to one or more media players to fetch music data.
pub trait Driver {
    /// Get currently playing song's info, if it exists.
    fn fetch_song_info(&mut self) -> Option<SongInfo>;
}

/// Factory for creating Driver implementations based on their names.
pub fn create(name: &str) -> Option<Box<dyn Driver>> {
    match name {
        "spotify-desktop" => Some(Box::new(SpotifyDesktopDriver::new())),
        _ => None,
    }
}
