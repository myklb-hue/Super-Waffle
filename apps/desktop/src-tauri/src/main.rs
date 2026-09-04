// The window is created on the main thread; everything else lives in the lib,
// so the host can also be built as a library for tests.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cyberloom_lib::run()
}
