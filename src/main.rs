use config::Config;
use crossbeam_channel::RecvTimeoutError;
use song::SongInfo;
use std::{fs, thread, time::Duration};
use sysinfo::{PidExt, ProcessExt, ProcessRefreshKind, System, SystemExt};

mod config;
mod process;
mod song;

fn main() {
    // Handle SIGINT, SIGTERM, etc. for graceful shutdown
    let (signal_sender, signal_receiver) = crossbeam_channel::unbounded::<()>();
    ctrlc::set_handler(move || signal_sender.send(()).expect("Cannot send signal"))
        .expect("Cannot set handler");

    let directory = dirs::config_dir().unwrap().join("Frixuu.CurrentSong");
    fs::create_dir_all(&directory).expect("Cannot create config dir");

    let config_path = directory.join("config.json");
    if !config_path.exists() {
        let default_config = Config::default();
        let default_config_json = serde_json::to_string(&default_config).unwrap();
        fs::write(&config_path, default_config_json).expect("Cannot write default config");
        // Since this is probably a first time run,
        // reveal the directory to show the user where we store the app's files
        open::that_in_background(&directory);
    }

    let config = fs::read_to_string(&config_path)
        .map(|json| serde_json::from_str::<Config>(&json).expect("Cannot parse config file"))
        .expect("Cannot read config file");

    // Create another thread for handling song information.
    // This should help with IO access times being unpredictable
    let (song_sender, song_receiver) = crossbeam_channel::unbounded::<Option<SongInfo>>();
    let writing_thread = thread::spawn(move || loop {
        match song_receiver.recv() {
            Ok(Some(song)) => {
                let format = config.song_format();
                let song_str = format
                    .replace("{artist}", &song.artist)
                    .replace("{title}", &song.title);
                println!("Now: {}", song_str);
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
