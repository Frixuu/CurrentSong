use std::thread::JoinHandle;

use flume::{SendError, Sender};

pub trait Actor {
    type MessageType;
    fn spawn(self) -> ActorHandle<Self::MessageType>;
}

pub struct ActorHandle<M> {
    pub sender: Sender<M>,
    pub thread_handle: JoinHandle<()>,
}

impl<M> ActorHandle<M> {
    pub fn send(&self, message: M) -> Result<(), SendError<M>> {
        self.sender.send(message)
    }
}
