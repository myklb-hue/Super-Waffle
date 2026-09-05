fn main() {
    // Tauri tells its dependents whether it was built for dev — which it is
    // whenever the `custom-protocol` feature is off, whatever the cargo
    // profile says. A release binary in dev mode loads the Vite dev server's
    // URL and shows "Could not connect to localhost" on every machine that is
    // not a developer's. That is a mistake to make at compile time, not at
    // first run.
    let release = std::env::var("PROFILE").is_ok_and(|p| p == "release");
    let dev = std::env::var("DEP_TAURI_DEV").is_ok_and(|d| d == "true");
    if release && dev {
        panic!(
            "a release build of the host without `--features custom-protocol` would load the \
             dev server instead of the bundled shell. Build it as \
             `cargo build --release --bin cyberloom --features custom-protocol`, or with \
             `npx @tauri-apps/cli@2 build`, which turns the feature on itself."
        );
    }
    tauri_build::build()
}
