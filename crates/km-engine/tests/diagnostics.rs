//! A catalogue rather than a claim.
//!
//! The bundled keyboards are compiled binaries, so "what does this key produce"
//! cannot be read off by eye. This test prints the answer for every bundled
//! keyboard — which keys produce Myanmar, and which short key sequences produce
//! an invisible character — so a mismatch against KeyMagic can be traced to a
//! sequence instead of a guess.
//!
//!     cargo test -p km-engine --test diagnostics -- --ignored --nocapture
//!
//! It is ignored by default because it asserts nothing: it is a map, and maps
//! belong in the log.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use km_core::{KeyInput, Modifiers};
use km_engine::{Engine, Keyboard};

/// What counts as an invisible character: a zero-width space, a non-joiner, a
/// joiner, or a byte-order mark. KeyMagic never types these on purpose; anything
/// here is either a rule that means it or a bug.
const FURNITURE: [char; 4] = ['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}'];

fn bundled() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../keyboards/bundled");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("{} is unreadable: {error}", root.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "km2"))
        .collect();
    found.sort();
    found
}

fn engine(path: &Path) -> Engine {
    let bytes = std::fs::read(path).expect("a bundled keyboard is readable");
    let keyboard = Keyboard::from_bytes(&bytes).expect("a bundled keyboard decodes");
    Engine::new(std::sync::Arc::new(keyboard))
}

/// Escapes the invisible characters so a log can show them.
fn show(text: &str) -> String {
    let mut shown = String::new();
    for character in text.chars() {
        if FURNITURE.contains(&character) {
            let _ = write!(shown, "<U+{:04X}>", character as u32);
        } else {
            shown.push(character);
        }
    }
    shown
}

#[test]
#[ignore = "prints a catalogue; run with --ignored --nocapture"]
fn what_the_bundled_keyboards_type() {
    let keyboards = bundled();
    assert!(!keyboards.is_empty(), "no bundled keyboards to look at");

    // The keys a Myanmar layout is written against, plus the punctuation that
    // sits where Myanmar vowels do on a US keyboard.
    let keys: Vec<char> = "abcdefghijklmnopqrstuvwxyz\
                           ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                           0123456789`-=[]\\;',./~!@#$%^&*()_+{}|:\"<>?"
        .chars()
        .collect();

    for path in keyboards {
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_default();
        println!("\n=== {name}");

        // One key at a time: which key produces which letter.
        let mut single = String::new();
        for key in &keys {
            let mut engine = engine(&path);
            engine.key(KeyInput::character(*key));
            let out = engine.context();
            if !out.is_empty() && !out.is_ascii() {
                let _ = writeln!(single, "  {key:>2} -> {}", show(out));
            }
        }
        println!("-- one key:{single}");

        // Two keys at a time: the shortest sequence that can show a reordering
        // or a joining decision, and where an invisible character would appear.
        let mut furniture = Vec::new();
        for first in &keys {
            for second in &keys {
                let mut engine = engine(&path);
                engine.key(KeyInput::character(*first));
                engine.key(KeyInput::character(*second));
                let out = engine.context();
                if out.chars().any(|c| FURNITURE.contains(&c)) {
                    furniture.push(format!("  {first}{second} -> {}", show(out)));
                }
            }
        }
        if furniture.is_empty() {
            println!("-- no two-key sequence produces an invisible character");
        } else {
            println!(
                "-- invisible characters from two keys:\n{}",
                furniture.join("\n")
            );
        }
    }

    // A holder so the unused import is used when the catalogue is compiled in a
    // build where nothing above needs it.
    let _ = Modifiers::default();
}

#[test]
#[ignore = "prints the rules for one vowel; run with --ignored --nocapture"]
fn what_pyidaungsu_says_about_the_vowel_e() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../keyboards/bundled")
        .join("pyidaungsu-mm.km2");
    let bytes = std::fs::read(&path).expect("a bundled keyboard is readable");
    let raw = km_binary::read(&bytes).expect("a bundled keyboard decodes");
    let keyboard = Keyboard::from_bytes(&bytes).expect("the engine decodes the keyboard");

    println!("\n=== pyidaungsu-mm word check");
    for sequence in ["ta;", "at;", "t;a"] {
        let mut engine = engine(&path);
        for key in sequence.chars() {
            engine.key(KeyInput::character(key));
        }
        let output = engine.context();
        let codepoints: Vec<String> = output
            .chars()
            .map(|character| format!("U+{:04X}", character as u32))
            .collect();
        println!("  {sequence} -> {} {codepoints:?}", show(output));
    }

    println!("\n=== pyidaungsu-mm, rules containing the vowel or invisible characters");
    for rule in &raw.rules {
        if rule.rhs.contains(&0x1031)
            || rule
                .rhs
                .iter()
                .any(|word| FURNITURE.contains(&char::from_u32(u32::from(*word)).unwrap_or(' ')))
        {
            println!(
                "  #{} lhs={:04X?} rhs={:04X?}",
                rule.index, rule.lhs, rule.rhs
            );
        }
    }
    println!("=== decoded engine rules that emit the vowel or invisible characters");
    for rule in &keyboard.rules {
        let interesting = rule.emits.iter().any(|emit| match emit {
            km_engine::Emit::Text(text) => {
                text.contains('ေ') || text.chars().any(|c| FURNITURE.contains(&c))
            }
            km_engine::Emit::Reference(_) | km_engine::Emit::Switch(_) => false,
            km_engine::Emit::Indexed { table, .. } => {
                table.contains('ေ') || table.chars().any(|c| FURNITURE.contains(&c))
            }
        });
        if interesting {
            println!("  #{} {} => {:?}", rule.index, rule.describe(), rule.emits);
        }
    }
}
