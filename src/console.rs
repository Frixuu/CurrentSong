use std::sync::Arc;

use crate::{config::Config, song::SongInfo, Actor, ActorHandle};

pub struct ConsoleActor {
    config: Arc<Config>,
}

impl ConsoleActor {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
}

impl Actor for ConsoleActor {
    type MessageType = Option<SongInfo>;
    fn spawn(self) -> ActorHandle<Self::MessageType> {
        let (sender, receiver) = flume::unbounded();
        ActorHandle {
            sender,
            thread_handle: std::thread::spawn(move || loop {
                match receiver.recv() {
                    Ok(Some(song)) => {
                        let format = self.config.song_format();
                        let song_str = format
                            .replace("{artist}", &song.artist)
                            .replace("{title}", &song.title);

                        println!("Now: {}", song_str);
                    }
                    Ok(None) => {
                        println!("Now: ---");
                    }
                    _ => break,
                }
            }),
        }
    }
}
