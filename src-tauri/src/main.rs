// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// See the note in lib.rs.
#![cfg_attr(windows, allow(linker_messages))]

fn main() {
  open_video_downloader_lib::run();
}
