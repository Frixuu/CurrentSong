#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use actor::{Actor, ActorHandle};
use app::AppBuilder;

mod actor;
mod app;
mod config;
mod console;
mod driver;
mod process;
mod song;
mod windowing;

fn main() {
    let app = AppBuilder::new().build();
    app.run();
}
