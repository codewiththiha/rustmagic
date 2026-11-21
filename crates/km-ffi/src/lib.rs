//! A C ABI, for the platform adapters that cannot call Rust directly.
//!
//! Windows has a TSF text service, macOS has an input method in Objective-C and
//! Linux has an IBus engine. All three want to talk to a plain C surface, so the
//! engine ships one rather than asking each adapter to link Rust.
//!
//! The rules of the surface, kept small on purpose:
//!
//! * every `rm_*` pointer is owned by the caller until the matching `_free`;
//! * every string this library returns is allocated by it and must be released
//!   with [`rm_string_free`], never with `free`;
//! * a null handle is never dereferenced: calls on it return a neutral answer;
//! * errors come back as a message string rather than an errno.
//!
//! Everything below is a thin translation of [`km_session`]-free engine calls:
//! no state lives here that the engine does not own.

#![allow(unsafe_code)]
#![allow(non_camel_case_types)]

use std::ffi::{CStr, CString, c_char, c_int};
use std::path::Path;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use km_core::{Error, KeyInput, Modifiers};
use km_engine::{Engine, Keyboard};

/// An opaque loaded keyboard.
pub struct rm_keyboard {
    keyboard: Arc<Keyboard>,
}

/// An opaque engine driving one composition.
pub struct rm_engine {
    engine: Engine,
}

/// The result of one key press, in UTF-8.
///
/// `erase` characters before the caret should be deleted, then `insert` written.
/// `insert` must be released with [`rm_string_free`], and is never null: use the
/// empty string to mean "nothing to write".
#[repr(C)]
pub struct rm_edit {
    /// Characters to delete before the caret.
    pub erase: usize,
    /// Text to write at the caret, owned by the caller.
    pub insert: *mut c_char,
    /// Whether the engine handled the key, so the host should not.
    pub consumed: c_int,
    /// Whether matching gave up after the recursion limit.
    pub recursion_limit: c_int,
}

static LAST_ERROR: AtomicU32 = AtomicU32::new(0);

fn owned(text: &str) -> *mut c_char {
    CString::new(text)
        .unwrap_or_else(|_| CString::new(text.replace('\0', "")).unwrap_or_default())
        .into_raw()
}

fn owned_or_empty() -> *mut c_char {
    owned("")
}

/// Releases a string this library returned.
///
/// # Safety
///
/// `text` must be a pointer this library returned and must not be freed twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_string_free(text: *mut c_char) {
    if !text.is_null() {
        // Reclaim the allocation the library made; `CString` owns it again.
        drop(unsafe { CString::from_raw(text) });
    }
}

/// The library version, as a static string that must not be freed.
#[unsafe(no_mangle)]
pub extern "C" fn rm_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

/// Loads a keyboard from a `.km2` or `.kms` file.
///
/// Returns null on failure and, when `error` is not null, stores a message there
/// that the caller must release with [`rm_string_free`].
///
/// # Safety
///
/// `path` must be a valid NUL-terminated string; `error` may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_keyboard_load(
    path: *const c_char,
    error: *mut *mut c_char,
) -> *mut rm_keyboard {
    let Ok(text) = (unsafe { CStr::from_ptr(path) }).to_str() else {
        return fail(error, "the path is not valid UTF-8");
    };
    match load(Path::new(text)) {
        Ok(keyboard) => {
            report(error, ptr::null_mut());
            Box::into_raw(Box::new(rm_keyboard {
                keyboard: Arc::new(keyboard),
            }))
        }
        Err(problem) => fail(error, &problem.to_string()),
    }
}

/// Loads a keyboard from `.km2` bytes already in memory.
///
/// # Safety
///
/// `bytes` must point at `len` readable bytes; `error` may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_keyboard_from_bytes(
    bytes: *const u8,
    len: usize,
    error: *mut *mut c_char,
) -> *mut rm_keyboard {
    if bytes.is_null() {
        return fail(error, "no bytes were given");
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    match Keyboard::from_bytes(slice) {
        Ok(keyboard) => {
            report(error, ptr::null_mut());
            Box::into_raw(Box::new(rm_keyboard {
                keyboard: Arc::new(keyboard),
            }))
        }
        Err(problem) => fail(error, &problem.to_string()),
    }
}

/// Releases a keyboard. The engines made from it keep working.
///
/// # Safety
///
/// `keyboard` must come from this library and must not be freed twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_keyboard_free(keyboard: *mut rm_keyboard) {
    if !keyboard.is_null() {
        drop(unsafe { Box::from_raw(keyboard) });
    }
}

/// The keyboard's name, owned by the caller.
///
/// # Safety
///
/// `keyboard` may be null, in which case an empty string is returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_keyboard_name(keyboard: *const rm_keyboard) -> *mut c_char {
    match unsafe { keyboard.as_ref() } {
        Some(keyboard) => owned(&keyboard.keyboard.meta.name),
        None => owned_or_empty(),
    }
}

/// The keyboard's description, owned by the caller.
///
/// # Safety
///
/// `keyboard` may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_keyboard_description(keyboard: *const rm_keyboard) -> *mut c_char {
    match unsafe { keyboard.as_ref() } {
        Some(keyboard) => owned(&keyboard.keyboard.meta.description),
        None => owned_or_empty(),
    }
}

/// The keyboard's hotkey, in the `CTRL+SHIFT+A` form, owned by the caller.
///
/// # Safety
///
/// `keyboard` may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_keyboard_hotkey(keyboard: *const rm_keyboard) -> *mut c_char {
    match unsafe { keyboard.as_ref() } {
        Some(keyboard) => owned(&keyboard.keyboard.meta.hotkey),
        None => owned_or_empty(),
    }
}

/// How many rules the keyboard has.
///
/// # Safety
///
/// `keyboard` may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_keyboard_rule_count(keyboard: *const rm_keyboard) -> usize {
    unsafe { keyboard.as_ref() }.map_or(0, |keyboard| keyboard.keyboard.rule_count())
}

/// Creates an engine for a keyboard.
///
/// # Safety
///
/// `keyboard` must come from this library and must outlive the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_engine_new(keyboard: *const rm_keyboard) -> *mut rm_engine {
    let Some(keyboard) = (unsafe { keyboard.as_ref() }) else {
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(rm_engine {
        engine: Engine::new(Arc::clone(&keyboard.keyboard)),
    }))
}

/// Releases an engine.
///
/// # Safety
///
/// `engine` must come from this library and must not be freed twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_engine_free(engine: *mut rm_engine) {
    if !engine.is_null() {
        drop(unsafe { Box::from_raw(engine) });
    }
}

/// Forgets the composition, keeping the rules and switches.
///
/// # Safety
///
/// `engine` may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_engine_reset(engine: *mut rm_engine) {
    if let Some(engine) = unsafe { engine.as_mut() } {
        engine.engine.reset();
    }
}

/// Replaces the composition with `text`.
///
/// # Safety
///
/// `text` must be a valid NUL-terminated string; it is copied.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_engine_set_context(engine: *mut rm_engine, text: *const c_char) {
    let (Some(engine), Ok(text)) = (
        (unsafe { engine.as_mut() }),
        unsafe { CStr::from_ptr(text) }.to_str(),
    ) else {
        return;
    };
    engine.engine.set_context(text);
}

/// The composition so far, owned by the caller.
///
/// # Safety
///
/// `engine` may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_engine_context(engine: *const rm_engine) -> *mut c_char {
    match unsafe { engine.as_ref() } {
        Some(engine) => owned(engine.engine.context()),
        None => owned_or_empty(),
    }
}

/// Feeds one key to the engine and reports what the host should write.
///
/// `ch` is the character the keyboard produced for the key, or 0 when the key
/// produces none. `out` may be null when the caller only wants the return value,
/// which is 1 when the engine consumed the key and 0 when the host should type it.
///
/// # Safety
///
/// `engine` must come from this library; `out`, when not null, must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_engine_key(
    engine: *mut rm_engine,
    key: u16,
    ch: u32,
    shift: c_int,
    control: c_int,
    alt: c_int,
    alt_gr: c_int,
    caps: c_int,
    out: *mut rm_edit,
) -> c_int {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return 0;
    };
    let input = KeyInput {
        key,
        text: char::from_u32(ch),
        modifiers: Modifiers {
            shift: shift != 0,
            control: control != 0,
            alt: alt != 0,
            alt_gr: alt_gr != 0,
            caps: caps != 0,
        },
    };
    let outcome = engine.engine.key(input);
    if let Some(out) = unsafe { out.as_mut() } {
        out.erase = outcome.edit.erase;
        out.insert = owned(&outcome.edit.insert);
        out.consumed = c_int::from(outcome.consumed);
        out.recursion_limit = c_int::from(outcome.recursion_limit);
    }
    c_int::from(outcome.consumed)
}

/// Asks the engine what it would do with a key, without doing it.
///
/// A Windows text service is asked twice about every key: once to decide whether
/// to claim it at all ([`ITfKeyEventSink::OnTestKeyDown`]) and once to process it.
/// Claiming a key it will not use makes the application unusable; failing to
/// claim one it would use means it never sees the keystroke. This answers the
/// first question by running the key against a copy of the engine's state and
/// throwing the copy away, so the context, the switches and the history are
/// exactly as they were.
///
/// Returns non-zero when the engine would consume the key.
///
/// # Safety
///
/// `engine` must be a handle from [`rm_engine_new`] or null, and must not be used
/// from two threads at once. Nothing is written through it.
///
/// [`ITfKeyEventSink::OnTestKeyDown`]: https://learn.microsoft.com/windows/win32/api/msctf/nf-msctf-itfkeyeventsink-ontestkeydown
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm_engine_key_test(
    engine: *const rm_engine,
    key: u16,
    ch: u32,
    shift: c_int,
    control: c_int,
    alt: c_int,
    alt_gr: c_int,
    caps: c_int,
) -> c_int {
    let Some(engine) = (unsafe { engine.as_ref() }) else {
        return 0;
    };
    let input = KeyInput {
        key,
        text: char::from_u32(ch),
        modifiers: Modifiers {
            shift: shift != 0,
            control: control != 0,
            alt: alt != 0,
            alt_gr: alt_gr != 0,
            caps: caps != 0,
        },
    };
    let mut probe = engine.engine.clone();
    c_int::from(probe.key(input).consumed)
}

fn load(path: &Path) -> Result<Keyboard, Error> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("kms") {
        let compiled = km_script::compile_file(path)?;
        Keyboard::from_raw(compiled.keyboard)
    } else {
        Keyboard::from_file(path)
    }
}

fn fail(error: *mut *mut c_char, message: &str) -> *mut rm_keyboard {
    report(error, owned(message));
    ptr::null_mut()
}

fn report(error: *mut *mut c_char, message: *mut c_char) {
    LAST_ERROR.fetch_add(1, Ordering::Relaxed);
    if !error.is_null() {
        unsafe { *error = message };
    } else if !message.is_null() {
        unsafe { rm_string_free(message) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_null_handle_answers_neutral() {
        unsafe {
            assert_eq!(rm_keyboard_rule_count(ptr::null()), 0);
            assert_eq!(
                rm_engine_key(ptr::null_mut(), 0x41, 0x61, 0, 0, 0, 0, 0, ptr::null_mut()),
                0
            );
            let text = rm_keyboard_name(ptr::null());
            assert_eq!(CStr::from_ptr(text).to_str().unwrap(), "");
            rm_string_free(text);
        }
    }

    #[test]
    fn asking_what_a_key_would_do_does_nothing() {
        // The question a Windows text service asks about every key press before
        // it claims the key, so it has to cost nothing: same answer as the real
        // pass, and the context, the switches and the history untouched.
        let path = CString::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../keyboards/bundled/pyidaungsu-mm.km2"
        ))
        .unwrap();
        let keyboard = unsafe { rm_keyboard_load(path.as_ptr(), ptr::null_mut()) };
        assert!(!keyboard.is_null(), "the bundled keyboard did not load");

        let asking = unsafe { rm_engine_new(keyboard) };
        let doing = unsafe { rm_engine_new(keyboard) };
        assert!(!asking.is_null() && !doing.is_null());

        // A syllable typed with the layout: "mingalarpar" is what a user types.
        let typed = [0x4D_u16, 0x49, 0x4E, 0x47, 0x41, 0x4C];
        for (index, key) in typed.iter().enumerate() {
            let ch = u32::from(b"mingal"[index]);
            let asked = unsafe { rm_engine_key_test(asking, *key, ch, 0, 0, 0, 0, 0) };
            let edited = unsafe { rm_engine_key(doing, *key, ch, 0, 0, 0, 0, 0, ptr::null_mut()) };
            assert_eq!(asked, edited, "key {key:#04x} disagreed with itself");
        }

        // The context the asking engine reports is still empty: nothing was
        // processed on it, even though the same keys changed the other engine.
        let asked_context = unsafe { rm_engine_context(asking) };
        let done_context = unsafe { rm_engine_context(doing) };
        let asked_text = unsafe { CStr::from_ptr(asked_context) }
            .to_str()
            .unwrap()
            .to_owned();
        let done_text = unsafe { CStr::from_ptr(done_context) }
            .to_str()
            .unwrap()
            .to_owned();
        assert_eq!(asked_text, "");
        assert_ne!(done_text, "", "the processing engine should have a context");
        unsafe {
            rm_string_free(asked_context);
            rm_string_free(done_context);
            rm_engine_free(asking);
            rm_engine_free(doing);
            rm_keyboard_free(keyboard);
        }
    }

    #[test]
    fn a_missing_file_reports_an_error_instead_of_crashing() {
        let path = CString::new("/nonexistent/x.km2").unwrap();
        let mut error: *mut c_char = ptr::null_mut();
        let keyboard = unsafe { rm_keyboard_load(path.as_ptr(), &raw mut error) };
        assert!(keyboard.is_null());
        assert!(!error.is_null());
        let message = unsafe { CStr::from_ptr(error) }
            .to_str()
            .unwrap()
            .to_owned();
        assert_ne!(message, "");
        unsafe { rm_string_free(error) };
    }

    #[test]
    fn the_whole_surface_round_trips_the_demo() {
        let asset = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/keyboards/demo-greek.kms"
        );
        let path = CString::new(asset).unwrap();
        unsafe {
            let keyboard = rm_keyboard_load(path.as_ptr(), ptr::null_mut());
            assert!(!keyboard.is_null());
            let name = rm_keyboard_name(keyboard);
            assert_eq!(CStr::from_ptr(name).to_str().unwrap(), "Demo Greek");
            rm_string_free(name);
            assert!(rm_keyboard_rule_count(keyboard) >= 4);

            let engine = rm_engine_new(keyboard);
            let mut edit = rm_edit {
                erase: 0,
                insert: ptr::null_mut(),
                consumed: 0,
                recursion_limit: 0,
            };
            let consumed =
                rm_engine_key(engine, 0x41, u32::from('a'), 0, 0, 0, 0, 0, &raw mut edit);
            assert_eq!(consumed, 1);
            assert_eq!(CStr::from_ptr(edit.insert).to_str().unwrap(), "α");
            rm_string_free(edit.insert);

            let context = rm_engine_context(engine);
            assert_eq!(CStr::from_ptr(context).to_str().unwrap(), "α");
            rm_string_free(context);

            rm_engine_free(engine);
            rm_keyboard_free(keyboard);
        }
    }
}
