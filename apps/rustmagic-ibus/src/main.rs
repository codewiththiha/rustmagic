//! The IBus engine process.
//!
//! IBus starts this with `--ibus` after reading the component file. Everything
//! interesting happens in `km_ibus`; this binary finds the state directory,
//! opens a session and hands both to the service.

#![deny(missing_docs)]

use std::path::PathBuf;
use std::process::ExitCode;

use km_session::{Host, Session};

/// The bus name the component file declares.
const BUS_NAME: &str = "org.freedesktop.IBus.RustMagic";
/// The engine name the panel shows.
const ENGINE_NAME: &str = "rustmagic";

/// The host an input method needs: folders, and a clipboard it may never use.
struct DaemonHost {
    state: PathBuf,
    cache: PathBuf,
}

impl Host for DaemonHost {
    fn state_dir(&self) -> PathBuf {
        self.state.clone()
    }

    fn cache_dir(&self) -> PathBuf {
        self.cache.clone()
    }

    fn set_clipboard(&self, _text: &str) {}

    fn open_external(&self, _url: &str) {}

    fn reveal(&self, _path: &std::path::Path) {}
}

fn main() -> ExitCode {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let state = std::env::var_os("XDG_STATE_HOME").map_or_else(
        || home.join(".local/state/rustmagic"),
        |base| PathBuf::from(base).join("rustmagic"),
    );
    let cache = std::env::var_os("XDG_CACHE_HOME").map_or_else(
        || home.join(".cache/rustmagic"),
        |base| PathBuf::from(base).join("rustmagic"),
    );
    let mut session = Session::open(Box::new(DaemonHost { state, cache }));
    session.seed_text(
        "demo-greek",
        include_str!("../../../assets/keyboards/demo-greek.kms"),
    );

    match async_io::block_on(km_ibus::serve(session, BUS_NAME, ENGINE_NAME)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rustmagic-ibus: {error}");
            ExitCode::FAILURE
        }
    }
}
