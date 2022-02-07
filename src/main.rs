use crossbeam_channel::RecvTimeoutError;
use song::SongInfo;
use std::{thread, time::Duration};
use sysinfo::{PidExt, ProcessExt, ProcessRefreshKind, System, SystemExt};

mod process;
mod song;

fn main() {
    // Handle SIGINT, SIGTERM, etc. for graceful shutdown
    let (signal_sender, signal_receiver) = crossbeam_channel::unbounded::<()>();
    ctrlc::set_handler(move || signal_sender.send(()).expect("Cannot send signal"))
        .expect("Cannot set handler");

    // Create another thread for handling song information.
    // This should help with IO access times being unpredictable
    let (song_sender, song_receiver) = crossbeam_channel::unbounded::<Option<SongInfo>>();
    let writing_thread = thread::spawn(move || loop {
        match song_receiver.recv() {
            Ok(Some(song)) => {
                println!("Now: {} by {}", song.title, song.artist);
            }
            Ok(None) => {
                println!("Now: ---");
            }
            _ => return,
        }
    });

    let mut system = System::new();
    let mut last_song: Option<SongInfo> = None;
    loop {
        // Update current process list
        system.refresh_processes_specifics(ProcessRefreshKind::new());

        // Try to get song info from Spotify window title
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

        // Check if the song changed
        if song != last_song {
            last_song = song.clone();
            song_sender.send(song).expect("Cannot send updated song");
        }

        // Exit if received signal or disconnected from channel
        match signal_receiver.recv_timeout(Duration::from_millis(2000)) {
            Err(RecvTimeoutError::Timeout) => {}
            _ => {
                drop(song_sender);
                break;
            }
        }
    }

    // Wait until all files get written to
    writing_thread.join().unwrap();
    println!("Closing!");
}
