#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app::AppBuilder;

mod app;
mod config;
mod driver;
mod process;
mod song;
mod windowing;

fn main() {
    let app = AppBuilder::new().build();
    app.run();
}
