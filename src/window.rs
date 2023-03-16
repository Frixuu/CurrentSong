use std::{
    cell::RefCell,
    ops::Deref,
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
};

use flume::{Receiver, Sender};
use nwg::{
    EmbedResource, Event, Font, GridLayout, Icon, Label, NativeUi, Notice, NwgError, Window,
    WindowFlags,
};

use crate::{app::LifecycleEvent, config::Config, song::SongInfo, Actor, ActorHandle};

pub struct WindowActor {
    sender: Sender<LifecycleEvent>,
    config: Arc<Config>,
}

impl WindowActor {
    pub fn new(sender: Sender<LifecycleEvent>, config: Arc<Config>) -> Self {
        Self { sender, config }
    }
}

impl Actor for WindowActor {
    type MessageType = Option<SongInfo>;
    fn spawn(self) -> ActorHandle<Self::MessageType> {
        let (s, r) = flume::unbounded();
        ActorHandle {
            sender: s,
            thread_handle: thread::spawn(move || {
                nwg::init().expect("cannot init NWG");
                Font::set_global_family("Segoe UI").expect("cannot set default font");
                let state: WindowApp = WindowApp {
                    config: self.config,
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
    config: Arc<Config>,
    window: Window,
    layout: GridLayout,
    label_artist: Label,
    label_title: Label,
    song_notice: Notice,
    current_song: Arc<Mutex<Option<SongInfo>>>,
    sender: Option<Sender<LifecycleEvent>>,
    receiver: Option<Receiver<Option<SongInfo>>>,
}

impl WindowApp {
    fn on_init(&self) {
        let song_receiver = self.receiver.clone().unwrap();
        let song_arc = self.current_song.clone();
        let notice_sender = self.song_notice.sender();
        notice_sender.notice();
        let _ = thread::spawn(move || loop {
            match song_receiver.recv() {
                Ok(data) => {
                    {
                        let mut song = song_arc.lock().unwrap();
                        *song = data;
                    }
                    notice_sender.notice();
                }
                Err(_) => break,
            }
        });
    }

    fn on_notice(&self) {
        let _config = self.config.as_ref();
        let song_arc = self.current_song.lock().unwrap();
        let song: Option<SongInfo> = song_arc.clone();
        match song {
            Some(song) => {
                self.label_artist.set_text(&song.artist);
                self.label_title.set_text(&song.title);
            }
            None => {
                self.label_artist.set_text("N/A");
                self.label_title.set_text("---");
            }
        }
    }

    fn on_window_close(&self) {
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

impl NativeUi<WindowUi> for WindowApp {
    fn build_ui(mut state: Self) -> Result<WindowUi, NwgError> {
        let embed = EmbedResource::load(None)?;
        Window::builder()
            .size((400, 120))
            .flags(WindowFlags::union(
                WindowFlags::union(WindowFlags::WINDOW, WindowFlags::VISIBLE),
                WindowFlags::MINIMIZE_BOX,
            ))
            .title("Current Song")
            .icon(Some(&Icon::from_embed(&embed, None, Some("ICON"))?))
            .build(&mut state.window)?;

        Notice::builder()
            .parent(&state.window)
            .build(&mut state.song_notice)?;

        Label::builder()
            .text("")
            .parent(&state.window)
            .build(&mut state.label_artist)?;

        Label::builder()
            .text("")
            .parent(&state.window)
            .build(&mut state.label_title)?;

        GridLayout::builder()
            .parent(&mut state.window)
            .max_row(Some(2))
            .spacing(5)
            .margin([30, 15, 30, 15])
            .child(0, 0, &state.label_artist)
            .child(0, 1, &state.label_title)
            .build(&state.layout)?;

        let ui = WindowUi {
            inner: Rc::new(state),
            default_handler: Default::default(),
        };

        let app = Rc::downgrade(&ui.inner);
        let handle_events = move |evt, _data, handle| {
            if let Some(app) = app.upgrade() {
                match evt {
                    Event::OnInit => app.on_init(),
                    Event::OnNotice => app.on_notice(),
                    Event::OnWindowClose => {
                        if &handle == &app.window {
                            app.on_window_close();
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

        return Ok(ui);
    }
}

impl Drop for WindowUi {
    fn drop(&mut self) {
        let handler = self.default_handler.borrow();
        if let Some(handler) = handler.as_ref() {
            nwg::unbind_event_handler(handler);
        }
    }
}

impl Deref for WindowUi {
    type Target = WindowApp;
    fn deref(&self) -> &WindowApp {
        &self.inner
    }
}
