use std::{
    fs::{self, File},
    path::PathBuf,
    sync::Arc,
};

use crate::{config::Config, song::SongInfo, Actor, ActorHandle};

pub struct FileWriterActor {
    config: Arc<Config>,
    path: PathBuf,
}

impl FileWriterActor {
    pub fn new(path: PathBuf, config: Arc<Config>) -> Self {
        Self { config, path }
    }
}

impl Actor for FileWriterActor {
    type MessageType = Option<SongInfo>;
    fn spawn(self) -> ActorHandle<Self::MessageType> {
        let (sender, receiver) = flume::unbounded();
        ActorHandle {
            sender,
            thread_handle: std::thread::spawn(move || {
                if let Err(err) = File::create(&self.path) {
                    eprintln!("  | Cannot create or truncate song.txt: {err:?}")
                }
                loop {
                    match receiver.recv() {
                        Ok(Some(song)) => {
                            let format = self.config.song_format();
                            let song_str = format
                                .replace("{artist}", &song.artist)
                                .replace("{title}", &song.title);
                            if let Err(err) = fs::write(&self.path, &song_str) {
                                eprintln!("  | Cannot save song.txt: {err:?}");
                            }
                        }
                        Ok(None) => {
                            let _ = File::create(&self.path);
                        }
                        _ => break,
                    }
                }
                let _ = File::create(&self.path);
            }),
        }
    }
}
