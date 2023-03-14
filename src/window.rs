use std::{cell::RefCell, ops::Deref, rc::Rc, sync::Arc};

use flume::{Receiver, Sender};
use nwg::{Event, NativeUi, WindowFlags};
use parking_lot::Mutex;

use crate::{app::LifecycleEvent, song::SongInfo, Actor, ActorHandle};

pub struct WindowActor {
    sender: Sender<LifecycleEvent>,
}

impl WindowActor {
    pub fn new(sender: Sender<LifecycleEvent>) -> Self {
        Self { sender }
    }
}

impl Actor for WindowActor {
    type MessageType = Option<SongInfo>;
    fn spawn(self) -> ActorHandle<Self::MessageType> {
        let (s, r) = flume::unbounded();
        ActorHandle {
            sender: s,
            thread_handle: std::thread::spawn(move || {
                nwg::init().expect("cannot init NWG");
                nwg::Font::set_global_family("Segoe UI").expect("cannot set default font");
                let state: WindowApp = WindowApp {
                    sender: Some(self.sender),
                    receiver: Some(r),
                    ..Default::default()
                };
                let _ui = WindowApp::build_ui(state).expect("cannot build UI");
                nwg::dispatch_thread_events();
            }),
        }
    }
}

#[derive(Default)]
struct WindowApp {
    window: nwg::Window,
    layout: nwg::GridLayout,
    label: nwg::Label,
    song_notice: nwg::Notice,
    current_song: Arc<Mutex<Option<SongInfo>>>,
    sender: Option<Sender<LifecycleEvent>>,
    receiver: Option<Receiver<Option<SongInfo>>>,
}

impl WindowApp {
    fn exit(&self) {
        nwg::stop_thread_dispatch();
        if let Some(sender) = &self.sender {
            sender
                .send(LifecycleEvent::Exit)
                .expect("cannot send exit event");
        }
    }
}

struct WindowUi {
    inner: Rc<WindowApp>,
    default_handler: RefCell<Option<nwg::EventHandler>>,
}

impl nwg::NativeUi<WindowUi> for WindowApp {
    fn build_ui(mut state: Self) -> Result<WindowUi, nwg::NwgError> {
        nwg::Window::builder()
            .size((400, 120))
            .flags(WindowFlags::union(
                WindowFlags::union(WindowFlags::WINDOW, WindowFlags::VISIBLE),
                WindowFlags::MINIMIZE_BOX,
            ))
            .title("Current Song")
            .build(&mut state.window)?;

        nwg::Notice::builder()
            .parent(&state.window)
            .build(&mut state.song_notice)?;

        nwg::Label::builder()
            .text("")
            .parent(&state.window)
            .build(&mut state.label)?;

        let ui = WindowUi {
            inner: Rc::new(state),
            default_handler: Default::default(),
        };

        let evt_ui = Rc::downgrade(&ui.inner);
        let handle_events = move |evt, _data, handle| {
            if let Some(evt_ui) = evt_ui.upgrade() {
                match evt {
                    Event::OnInit => {
                        let song_receiver = evt_ui.receiver.clone().unwrap();
                        let song_arc = evt_ui.current_song.clone();
                        let notice_sender = evt_ui.song_notice.sender();
                        notice_sender.notice();
                        let _ = std::thread::spawn(move || loop {
                            match song_receiver.recv() {
                                Ok(data) => {
                                    {
                                        let mut song = song_arc.lock();
                                        *song = data;
                                    }
                                    notice_sender.notice();
                                }
                                Err(_) => {}
                            }
                        });
                    }

                    Event::OnNotice => {
                        let song_arc = evt_ui.current_song.lock();
                        let song: Option<SongInfo> = song_arc.clone();
                        match song {
                            Some(song) => {
                                evt_ui
                                    .label
                                    .set_text(&format!("{} - {}", song.artist, song.title));
                            }
                            None => {
                                evt_ui.label.set_text("---");
                            }
                        }
                    }
                    Event::OnWindowClose => {
                        if &handle == &evt_ui.window {
                            WindowApp::exit(&evt_ui);
                        }
                    }
                    _ => {}
                }
            }
        };

        *ui.default_handler.borrow_mut() = Some(nwg::full_bind_event_handler(
            &ui.window.handle,
            handle_events,
        ));

        nwg::GridLayout::builder()
            .parent(&ui.window)
            .max_row(Some(1))
            .spacing(5)
            .margin([15, 15, 15, 15])
            .child(0, 0, &ui.inner.label)
            .build(&ui.inner.layout)?;

        return Ok(ui);
    }
}

impl Drop for WindowUi {
    fn drop(&mut self) {
        let handler = self.default_handler.borrow();
        if handler.is_some() {
            nwg::unbind_event_handler(handler.as_ref().unwrap());
        }
    }
}

impl Deref for WindowUi {
    type Target = WindowApp;
    fn deref(&self) -> &WindowApp {
        &self.inner
    }
}
