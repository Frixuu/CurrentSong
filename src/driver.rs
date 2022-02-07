use crate::process;
use crate::song::SongInfo;
use sysinfo::{PidExt, ProcessExt, ProcessRefreshKind, System, SystemExt};

/// Driver connects to one or more media players to fetch music data.
pub trait Driver {
    /// Get currently playing song's info, if it exists.
    fn fetch_song_info(&mut self) -> Option<SongInfo>;
}

/// Factory for creating Driver implementations based on their names.
pub fn create(name: &str) -> Option<Box<dyn Driver>> {
    match name {
        "spotify-desktop" => Some(Box::new(SpotifyDesktop::new())),
        _ => None,
    }
}

pub struct SpotifyDesktop {
    system: System,
}

impl SpotifyDesktop {
    pub fn new() -> SpotifyDesktop {
        SpotifyDesktop {
            system: System::new(),
        }
    }
}

impl Driver for SpotifyDesktop {
    fn fetch_song_info(&mut self) -> Option<SongInfo> {
        let system = &mut self.system;
        system.refresh_processes_specifics(ProcessRefreshKind::new());
        let mut song: Option<SongInfo> = None;
        for process in system.processes_by_name("Spotify") {
            let pid = process.pid().as_u32();
            if let Some(window_title) = process::find_main_window_title(pid) {
                if !window_title.starts_with("Spotify") {
                    let mut parts = window_title.splitn(2, " - ");
                    let artist = parts.next().unwrap();
                    let title = parts.next().unwrap();
                    song = Some(SongInfo::new(artist.into(), title.into()));
                    break;
                }
            }
        }
        song
    }
}
