//! Container-level tests against files produced by the upstream compiler.

use km_binary::{RawRule, Version, read, write};
use km_core::LayoutOptions;

const MINIMAL: &[u8] = include_bytes!("../../../fixtures/minimal.km2");
const BASIC: &[u8] = include_bytes!("../../../fixtures/basic.km2");
const ZAWGYI: &[u8] = include_bytes!("../../../fixtures/zawgyi-l-unicode.km2");

/// Upstream leaves the header's padding byte uninitialised; it carries no data.
fn without_padding(bytes: &[u8]) -> Vec<u8> {
    let mut copy = bytes.to_vec();
    if copy.len() > 17 {
        copy[17] = 0;
    }
    copy
}

#[test]
fn reads_a_minimal_keyboard() {
    let keyboard = read(MINIMAL).expect("minimal file reads");
    assert_eq!(keyboard.version, Version::new(1, 5));
    assert_eq!(keyboard.strings.len(), 0);
    assert_eq!(keyboard.infos.len(), 0);
    assert_eq!(keyboard.rules.len(), 2);
    assert_eq!(keyboard.options, LayoutOptions::DEFAULT);
    assert_eq!(keyboard.warnings, [] as [km_binary::Warning; 0]);
}

#[test]
fn reads_options_and_tables() {
    let keyboard = read(BASIC).expect("basic file reads");
    assert_eq!(keyboard.strings.len(), 6);
    assert_eq!(keyboard.strings[0], vec![0x1000, 0x1001, 0x1002]);
    assert_eq!(keyboard.infos.len(), 4);
    let meta = keyboard.meta();
    assert_eq!(meta.name, "Fixture Basic");
    assert_eq!(meta.hotkey, "CTRL+SHIFT+M");
    assert!(keyboard.options.auto_bksp && keyboard.options.right_alt);
    assert!(!keyboard.options.eat && !keyboard.options.pos_based);
}

#[test]
fn rules_keep_their_opcode_words() {
    let keyboard = read(BASIC).expect("basic file reads");
    let null_rule: &RawRule = keyboard
        .rules
        .iter()
        .find(|rule| rule.rhs == vec![0x00F3, 0x0001])
        .expect("the VK_BACK rule is present");
    assert_eq!(null_rule.lhs, vec![0x00F6, 0x00F3, 0x0002]);
    assert_eq!(null_rule.rhs, vec![0x00F3, 0x0001]);
}

#[test]
fn writing_reproduces_the_upstream_bytes() {
    for file in [MINIMAL, BASIC] {
        let keyboard = read(file).expect("file reads");
        let ours = write(&keyboard).expect("file writes");
        assert_eq!(without_padding(&ours), without_padding(file));
    }
}

#[test]
fn an_old_writer_header_is_repaired_with_a_warning() {
    let keyboard = read(ZAWGYI).expect("zawgyi file reads");
    assert_eq!(keyboard.version, Version::new(1, 4));
    assert_eq!(keyboard.strings.len(), 12);
    assert_eq!(keyboard.infos.len(), 5);
    assert_eq!(keyboard.rules.len(), 232);
    assert_eq!(keyboard.meta().name, "ZawgyiL Uni");
    let warning = keyboard
        .warnings
        .first()
        .expect("the repair is reported, not hidden");
    assert!(warning.message.contains("info"));
}

#[test]
fn truncated_input_is_an_error_not_a_panic() {
    for length in 0..MINIMAL.len() {
        let cut = &MINIMAL[..length];
        let result = read(cut);
        assert!(result.is_err(), "prefix of {length} bytes must be rejected");
    }
}

#[test]
fn a_wrong_magic_code_is_named() {
    let mut file = MINIMAL.to_vec();
    file[0] = b'X';
    assert_eq!(read(&file), Err(km_core::Error::NotKeyMagic));
}

#[test]
fn a_newer_version_is_refused() {
    let mut file = MINIMAL.to_vec();
    file[5] = 9;
    assert!(matches!(
        read(&file),
        Err(km_core::Error::UnsupportedVersion { minor: 9, .. })
    ));
}
