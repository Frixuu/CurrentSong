use config::Config;
use flume::{RecvError, RecvTimeoutError};
use song::SongInfo;
use std::{
    fs::{self, File},
    sync::Arc,
    thread,
    time::Duration,
};

use crate::windowing::WindowEvent;

mod config;
mod driver;
mod process;
mod song;
mod windowing;

fn main() {
    // Handle SIGINT, SIGTERM, etc. for graceful shutdown
    let (signal_sender, signal_receiver) = flume::unbounded::<()>();

    let ssender = signal_sender.clone();
    ctrlc::set_handler(move || ssender.send(()).expect("Cannot send signal"))
        .expect("Cannot set handler");

    // Ensure our data directory exists
    let directory = dirs::config_dir().unwrap().join("Frixuu.CurrentSong");
    fs::create_dir_all(&directory).expect("Cannot create config dir");

    let config_path = directory.join("config.json");
    let config = Arc::new(Config::try_read(&config_path).unwrap_or_else(|_| {
        // Reveal the directory to show the user where we store the app's files
        open::that_in_background(&directory);
        println!("Trying to write a new config file to {:?}", &config_path);
        let config = Config::default();
        config
            .try_save(&config_path)
            .expect("Cannot save config file");

        // Saved successfully, let's continue
        config
    }));

    // Create another thread for handling song information.
    // This should help with IO access times being unpredictable
    let (song_sender, song_receiver) = flume::unbounded::<Option<SongInfo>>();
    let writing_config = config.clone();
    let writing_thread = thread::spawn(move || {
        let config = writing_config;

        // Ensure song info file exists
        let song_file_path = &directory.join("song.txt");
        if let Err(err) = File::create(&song_file_path) {
            eprintln!("  | Cannot create or truncate song.txt: {err:?}")
        }

        loop {
            match song_receiver.recv() {
                Ok(Some(song)) => {
                    let format = config.song_format();
                    let song_str = format
                        .replace("{artist}", &song.artist)
                        .replace("{title}", &song.title);

                    println!("Now: {}", song_str);
                    if let Err(err) = fs::write(&song_file_path, &song_str) {
                        eprintln!("  | Cannot save song.txt: {err:?}");
                    }
                }
                Ok(None) => {
                    println!("Now: ---");
                    let _ = File::create(&song_file_path);
                }
                _ => break,
            }
        }

        // Clear the song file on exit to mimic other apps like this
        let _ = File::create(&song_file_path);
    });

    let app_sender = signal_sender.clone();
    let window_thread = thread::spawn(move || {
        let r = windowing::create();
        loop {
            match r.recv() {
                Err(RecvError::Disconnected) | Ok(WindowEvent::Closed) => {
                    app_sender.send(()).unwrap();
                    break;
                }
            }
        }
    });

    let mut driver = driver::create(config.driver_name()).expect("Unknown driver name");
    let mut last_song: Option<SongInfo> = None;

    loop {
        let song = driver.fetch_song_info();

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
    println!("Closing!");
    writing_thread.join().unwrap();
}
