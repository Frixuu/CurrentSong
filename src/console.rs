use std::sync::Arc;

use flume::Receiver;

use crate::{config::Config, song::SongInfo, Actor, ActorHandle};

pub struct ConsoleActor {
    receiver: Option<Receiver<Option<SongInfo>>>,
    config: Arc<Config>,
}

impl ConsoleActor {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            receiver: None,
            config,
        }
    }

    fn receive(&mut self) -> Result<bool, ()> {
        match &self.receiver {
            None => Err(()),
            Some(receiver) => match receiver.recv() {
                Ok(Some(song)) => {
                    let format = self.config.song_format();
                    let song_str = format
                        .replace("{artist}", &song.artist)
                        .replace("{title}", &song.title);

                    println!("Now: {}", song_str);
                    Ok(true)
                }
                Ok(None) => {
                    println!("Now: ---");
                    Ok(true)
                }
                _ => Ok(false),
            },
        }
    }
}

impl Actor for ConsoleActor {
    type MessageType = Option<SongInfo>;
    fn spawn(mut self) -> ActorHandle<Self::MessageType> {
        let (s, r) = flume::unbounded();
        self.receiver = Some(r);
        ActorHandle {
            sender: s,
            thread_handle: std::thread::spawn(move || loop {
                match self.receive() {
                    Ok(true) => {}
                    Ok(false) => break,
                    Err(_) => {
                        eprintln!("error in ConsoleActor while receiving message");
                        break;
                    }
                }
            }),
        }
    }
}
