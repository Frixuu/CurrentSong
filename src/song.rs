#[derive(PartialEq, Clone)]
pub struct SongInfo {
    pub artist: String,
    pub title: String,
}

impl SongInfo {
    pub fn new(artist: String, title: String) -> SongInfo {
        SongInfo { artist, title }
    }
}
