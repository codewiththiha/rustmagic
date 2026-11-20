//! Compiling the fixtures and comparing with what the upstream compiler wrote.

use std::path::{Path, PathBuf};

use km_script::{compile, compile_file};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

/// Upstream leaves the header's padding byte uninitialised.
fn normalised(mut bytes: Vec<u8>) -> Vec<u8> {
    if bytes.len() > 17 {
        bytes[17] = 0;
    }
    bytes
}

#[test]
fn basic_fixture_matches_upstream_bytes() {
    let compiled = compile_file(fixture("basic.kms")).expect("fixture compiles");
    let golden = std::fs::read(fixture("basic.km2")).expect("golden file exists");
    assert_eq!(
        normalised(compiled.to_bytes().expect("writes")),
        normalised(golden)
    );
}

#[test]
fn minimal_fixture_matches_upstream_bytes() {
    let compiled = compile_file(fixture("minimal.kms")).expect("fixture compiles");
    let golden = std::fs::read(fixture("minimal.km2")).expect("golden file exists");
    assert_eq!(
        normalised(compiled.to_bytes().expect("writes")),
        normalised(golden)
    );
}

#[test]
fn options_reach_the_header_and_the_info_table() {
    let compiled = compile_file(fixture("basic.kms")).expect("fixture compiles");
    let options = compiled.keyboard.options;
    assert!(options.track_caps, "@TRACK_CAPSLOCK = TRUE");
    assert!(options.auto_bksp, "@SMART_BACKSPACE = TRUE");
    assert!(options.right_alt, "@TREAT_CTRL_ALT_AS_RALT = TRUE");
    assert!(!options.eat, "@EAT_ALL_UNUSED_KEYS = FALSE");
    let meta = compiled.keyboard.meta();
    assert_eq!(meta.name, "Fixture Basic");
    assert_eq!(meta.font_family, "Inter");
    assert_eq!(compiled.stats.infos, 4);
    assert!(compiled.warnings.is_empty(), "{:?}", compiled.warnings);
}

#[test]
fn switches_and_variables_share_the_string_table() {
    let compiled = compile_file(fixture("basic.kms")).expect("fixture compiles");
    let strings = &compiled.keyboard.strings;
    assert_eq!(strings.len(), 6);
    assert_eq!(strings[1], vec![0x00F1, 1, 0x0078, 0x0079, 0x007A]);
    assert_eq!(strings[5], vec![0x004B], "a switch stores one placeholder");
}

#[test]
fn a_unicode_point_above_the_basic_plane_uses_two_units() {
    let source = "\"\u{1F600}\" => U1000\n";
    let compiled = compile(source, "<memory>", None).expect("compiles");
    assert_eq!(
        compiled.keyboard.rules[0].lhs,
        vec![0x00F0, 2, 0xD83D, 0xDE00]
    );
}

#[test]
fn includes_are_compiled_in_place() {
    let dir = std::env::temp_dir().join("rustmagic-include-test");
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("part.kms"), "\"x\" => U1001\n").expect("part written");
    std::fs::write(
        dir.join("main.kms"),
        "\"a\" => U1000\ninclude(\"part.kms\")\n\"y\" => U1002\n",
    )
    .expect("main written");
    let compiled = compile_file(dir.join("main.kms")).expect("compiles");
    assert_eq!(compiled.stats.rules, 3);
    assert_eq!(compiled.keyboard.rules[1].lhs, vec![0x00F0, 1, 0x0078]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_include_names_the_file() {
    let error = compile(
        "include(\"nope.kms\")\n",
        "<memory>",
        Some(Path::new("/tmp")),
    );
    let message = error.expect_err("must fail").to_string();
    assert!(message.contains("nope.kms"), "{message}");
}

#[test]
fn errors_name_the_position() {
    for (source, needle) in [
        ("\"a\" => $0\n", "starts at `$1`"),
        ("$missing => \"a\"\n", "used before it is defined"),
        ("$dup = \"a\"\n$dup = \"b\"\n", "already defined"),
        ("\"unterminated", "unterminated string"),
        ("<VK_BACK> + \"a\" => \"b\"\n", "nothing may follow them"),
        ("$v = \"abc\"\n$v[*] => \"x\"\n\"a\" + \"b\" => \"c\"\n", ""),
        ("\"a\" =>\n", "expected a right-side segment"),
        ("\"a\" => \"b\n", "a string cannot span lines"),
        ("'a' => VK_NOPE\n", "is not a known name"),
        ("'a' => $v\n", "used before it is defined"),
    ] {
        let result = compile(source, "test.kms", None);
        if needle.is_empty() {
            assert!(result.is_ok(), "{source:?} should compile");
            continue;
        }
        let error = result.expect_err(source).to_string();
        assert!(
            error.contains(needle),
            "{source:?} produced {error:?}, wanted {needle:?}"
        );
        assert!(error.contains("test.kms:"), "{error} names no file");
    }
}

#[test]
fn a_reference_past_the_left_side_warns_but_compiles() {
    let compiled = compile("\"a\" => $2\n", "test.kms", None).expect("compiles");
    assert_eq!(compiled.warnings.len(), 1);
    assert!(compiled.warnings[0].message.contains("$2"));
}

#[test]
fn every_line_of_a_truncated_source_is_rejected_cleanly() {
    let source = std::fs::read_to_string(fixture("basic.kms")).expect("fixture readable");
    for length in 0..source.len() {
        if !source.is_char_boundary(length) {
            continue;
        }
        let _ = compile(&source[..length], "test.kms", None);
    }
}
