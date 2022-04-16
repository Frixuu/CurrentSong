use crate::song::SongInfo;

use super::Driver;

/// A [Driver] that does nothing.
pub struct NoopDriver {}

impl Driver for NoopDriver {
    fn fetch_song_info(&mut self) -> Option<SongInfo> {
        None
    }
}

/// Creates a new [Driver] that does nothing.
pub fn noop() -> Box<NoopDriver> {
    Box::new(NoopDriver {})
}
