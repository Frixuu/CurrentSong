use std::{
    fs::{self, File},
    path::PathBuf,
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

use flume::{Receiver, RecvError, RecvTimeoutError, Sender};

use crate::{
    config::Config,
    driver::{self, Driver},
    song::SongInfo,
    windowing::{self, Window, WindowEvent},
};

pub enum LifecycleEvent {
    Exit,
}

pub struct App {
    /// Path to the directory where this app holds its data.
    data_directory: PathBuf,
    /// Configuration of the application.
    config: Arc<Config>,
    /// A template for a lifecycle sender.
    /// The user would typically clone it and pass it to a different thread
    /// to influence the behavior of the application.
    lifecycle_sender: Sender<LifecycleEvent>,
    lifecycle_receiver: Receiver<LifecycleEvent>,
    /// Thread that manages writing song data to disk, if one exists.
    thread_file_io: Option<JoinHandle<()>>,
    /// Thread that manages the main window of the application, if one exists.
    thread_window: Option<JoinHandle<()>>,
    /// The driver for resolving current song data.
    driver: Box<dyn Driver>,
    /// Time interval between requesting song information.
    duration_polling: Duration,
}

trait AppModule {
    fn start(app: &mut App);
    fn stop(app: &mut App);
}

/// A helper object for creating the application.
pub struct AppBuilder {}

impl AppBuilder {
    /// Creates a new AppBuilder with the default configuration.
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(self) -> App {
        let data_directory = dirs::config_dir().unwrap().join("Frixuu.CurrentSong");
        fs::create_dir_all(&data_directory).expect("cannot create config directory");

        let (s, r) = flume::unbounded::<LifecycleEvent>();
        let mut app = App {
            data_directory,
            config: Arc::new(Config::default()),
            lifecycle_sender: s,
            lifecycle_receiver: r,
            thread_file_io: None,
            thread_window: None,
            driver: driver::noop(),
            duration_polling: Duration::from_millis(1500),
        };

        app.load_config();
        app.load_driver();
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

        if let Some(thread_window) = self.thread_window {
            thread_window.thread().unpark();
        }
    }

    fn load_config(&mut self) {
        const CONFIG_FILE_NAME: &'static str = "config.json";
        let config_path = self.data_directory.join(CONFIG_FILE_NAME);
        let config = match Config::try_read(&config_path) {
            Ok(cfg) => cfg,
            Err(_) => {
                println!("Trying to write a new config file to {:?}", &config_path);
                let config = Config::default();
                config
                    .try_save(&config_path)
                    .expect("Cannot save config file");

                open::that_in_background(&self.data_directory);
                config
            }
        };

        self.config = Arc::new(config);
    }

    fn load_driver(&mut self) {
        let driver_name = self.config.driver_name();
        self.driver = driver::create(&driver_name).unwrap_or_else(|| {
            eprintln!("  | Unknown driver name: \"{}\"", &driver_name);
            driver::noop()
        });
    }

    /// Runs the application.
    /// This method exits only if the app has been gracefully shut down.
    pub fn run(mut self) {
        // Create another thread for handling song information.
        // This should help with IO access times being unpredictable
        let (song_sender, song_receiver) = flume::unbounded::<Option<SongInfo>>();
        let writing_config = self.config.clone();
        let writing_dir = self.data_directory.clone();
        self.thread_file_io = Some(thread::spawn(move || {
            let config = writing_config;

            // Ensure song info file exists
            let song_file_path = writing_dir.join("song.txt");
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
        self.thread_window = Some(thread::spawn(move || {
            let (window, r) = windowing::create();
            thread::sleep(Duration::from_millis(2000));
            window.run_on_ui_thread(move || {
                println!("eo");
            });
            loop {
                match r.recv() {
                    Err(RecvError::Disconnected) | Ok(WindowEvent::Closed) => {
                        app_sender.send(LifecycleEvent::Exit).unwrap();
                        break;
                    }
                }
            }
        }));

        let mut last_song: Option<SongInfo> = None;

        let lifecycle_receiver = self.lifecycle_receiver.clone();
        loop {
            let song = self.driver.fetch_song_info();

            // Check if the song changed
            if song != last_song {
                last_song = song.clone();
                song_sender.send(song).expect("Cannot send updated song");
            }

            match lifecycle_receiver.recv_timeout(self.duration_polling) {
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
