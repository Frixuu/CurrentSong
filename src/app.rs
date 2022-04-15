use std::{
    fs::{self, File},
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

use flume::{Receiver, RecvError, RecvTimeoutError, Sender};

use crate::{
    config::Config,
    driver,
    song::SongInfo,
    windowing::{self, WindowEvent},
};

pub enum LifecycleEvent {
    Exit,
}

pub struct App {
    lifecycle_sender: Sender<LifecycleEvent>,
    lifecycle_receiver: Receiver<LifecycleEvent>,
    thread_file_io: Option<JoinHandle<()>>,
}

pub struct AppBuilder {}

impl AppBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(self) -> App {
        let (s, r) = flume::unbounded::<LifecycleEvent>();
        let mut app = App {
            lifecycle_sender: s,
            lifecycle_receiver: r,
            thread_file_io: None,
        };
        app.setup_interrupts();
        app
    }
}

impl App {
    /// Registers SIGINT and SIGTERM listeners for graceful shutdown invocation.
    fn setup_interrupts(&mut self) {
        let sender = self.lifecycle_sender.clone();
        ctrlc::set_handler(move || {
            sender
                .send(LifecycleEvent::Exit)
                .expect("cannot send signal")
        })
        .expect("cannot set handler");
    }

    fn clean_up_before_exit(self) {
        if let Some(thread_handle) = self.thread_file_io {
            thread_handle.join().unwrap();
        }
    }

    /// Runs the application.
    /// This method exits only if the app has been gracefully shut down.
    pub fn run(mut self) {
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
        self.thread_file_io = Some(thread::spawn(move || {
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
        }));

        let app_sender = self.lifecycle_sender.clone();
        let _window_thread = thread::spawn(move || {
            let r = windowing::create();
            loop {
                match r.recv() {
                    Err(RecvError::Disconnected) | Ok(WindowEvent::Closed) => {
                        app_sender.send(LifecycleEvent::Exit).unwrap();
                        break;
                    }
                }
            }
        });

        let mut driver = driver::create(config.driver_name()).expect("Unknown driver name");
        let mut last_song: Option<SongInfo> = None;

        let lifecycle_receiver = self.lifecycle_receiver.clone();
        loop {
            let song = driver.fetch_song_info();

            // Check if the song changed
            if song != last_song {
                last_song = song.clone();
                song_sender.send(song).expect("Cannot send updated song");
            }

            match lifecycle_receiver.recv_timeout(Duration::from_millis(2000)) {
                Err(RecvTimeoutError::Timeout) => {
                    // Timeout is an expected result and not an error
                }
                Err(RecvTimeoutError::Disconnected) | Ok(LifecycleEvent::Exit) => {
                    drop(song_sender);
                    break;
                }
            }
        }

        self.clean_up_before_exit();
    }
}