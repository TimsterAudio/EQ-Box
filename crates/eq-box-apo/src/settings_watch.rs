//! Keeps the engine's settings in step with the file the EQ Box window
//! writes. One thread per process, shared by every engine instance.

use crate::status;
use eq_core::dsp::EqSettings;
use eq_core::engine_settings;
use eq_core::shared::Shared;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime};

const POLL: Duration = Duration::from_millis(100);
/// Write the status file every this many polls (once a second).
const STATUS_EVERY: u32 = 10;

static SHARED: OnceLock<Arc<Shared>> = OnceLock::new();

/// The settings every engine instance reads, starting the watcher the first
/// time it's asked for.
pub fn shared() -> Arc<Shared> {
    SHARED
        .get_or_init(|| {
            let path = engine_settings::default_path();
            let initial = match engine_settings::read_checked(&path) {
                Ok(s) => {
                    status::SETTINGS_READS.fetch_add(1, Relaxed);
                    s
                }
                Err(e) => {
                    status::SETTINGS_UNREADABLE.fetch_add(1, Relaxed);
                    status::set_read_error(&e);
                    EqSettings::default()
                }
            };
            let shared = Arc::new(Shared::new(&initial));
            let watched = shared.clone();
            // If the thread can't start, the engine still runs on the
            // settings it read just now.
            let _ = std::thread::Builder::new()
                .name("eq-box-settings".into())
                .spawn(move || watch(path, watched));
            shared
        })
        .clone()
}

fn watch(path: std::path::PathBuf, shared: Arc<Shared>) {
    let status_path = path.with_file_name(status::FILE_NAME);
    let mut seen: Option<SystemTime> = modified(&path);
    let mut ticks = 0u32;
    loop {
        std::thread::sleep(POLL);
        ticks += 1;
        if ticks.is_multiple_of(STATUS_EVERY) {
            // Best effort: diagnostics must never disturb the audio. If the
            // settings folder refuses the write, fall back to this
            // process's own temp folder.
            let report = status::report(&shared, &path);
            if let Err(e) = std::fs::write(&status_path, &report) {
                let fallback = std::env::temp_dir().join(status::FILE_NAME);
                let note = format!("could not write {}: {e}\r\n", status_path.display());
                let _ = std::fs::write(fallback, report + &note);
            }
        }
        let now = modified(&path);
        if now == seen {
            continue;
        }
        // A half-written or unreadable file is skipped and retried on the
        // next tick, keeping the current settings meanwhile.
        match engine_settings::read_checked(&path) {
            Ok(s) => {
                shared.store(&s);
                status::SETTINGS_READS.fetch_add(1, Relaxed);
                seen = now;
            }
            Err(_) if now.is_none() => {
                // The file was deleted: go flat rather than keep a stale EQ.
                shared.store(&EqSettings::default());
                seen = now;
            }
            Err(e) => {
                status::SETTINGS_UNREADABLE.fetch_add(1, Relaxed);
                status::set_read_error(&e);
            }
        }
    }
}

fn modified(path: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}
