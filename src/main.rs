use crossbeam_channel::TryRecvError;
use std::{thread, time::Duration};
use sysinfo::{PidExt, ProcessExt, System, SystemExt};

mod process;

fn main() {
    let (s, r) = crossbeam_channel::unbounded::<()>();
    ctrlc::set_handler(move || s.send(()).expect("Cannot send signal"))
        .expect("Cannot set handler");

    let mut system = System::new();
    loop {
        system.refresh_all();
        for process in system.processes_by_name("Spotify") {
            let pid = process.pid().as_u32();
            if let Some(title) = process::find_main_window_title(pid) {
                println!("{}", title);
            }
        }
        if let Err(TryRecvError::Empty) = r.try_recv() {
            thread::sleep(Duration::from_millis(2000));
        } else {
            break;
        }
    }

    println!("Goodbye!");
}
