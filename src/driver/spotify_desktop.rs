use crate::process;
use crate::song::SongInfo;
use sysinfo::{PidExt, ProcessExt, ProcessRefreshKind, System, SystemExt};

use super::Driver;

/// A [Driver] that fetches song information
/// from a locally installed Spotify app (free or premium).
pub struct SpotifyDesktopDriver {
    system: System,
}

impl SpotifyDesktopDriver {
    pub fn new() -> SpotifyDesktopDriver {
        SpotifyDesktopDriver {
            system: System::new(),
        }
    }
}

impl Driver for SpotifyDesktopDriver {
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
