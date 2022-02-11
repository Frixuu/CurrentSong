use config::Config;
use crossbeam_channel::RecvTimeoutError;
use song::SongInfo;
use std::{fs, sync::Arc, thread, time::Duration};

mod config;
mod driver;
mod process;
mod song;

fn main() {
    // Handle SIGINT, SIGTERM, etc. for graceful shutdown
    let (signal_sender, signal_receiver) = crossbeam_channel::unbounded::<()>();
    ctrlc::set_handler(move || signal_sender.send(()).expect("Cannot send signal"))
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
    let (song_sender, song_receiver) = crossbeam_channel::unbounded::<Option<SongInfo>>();
    let writing_config = config.clone();
    let writing_thread = thread::spawn(move || {
        let config = writing_config;
        loop {
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
    writing_thread.join().unwrap();
    println!("Closing!");
}
