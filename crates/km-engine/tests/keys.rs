//! Behaviour of the engine, checked against what upstream KeyMagic does.

use std::sync::Arc;

use km_core::{KeyInput, Modifiers};
use km_engine::{Engine, Keyboard};

/// Builds an engine from layout source.
fn engine(source: &str) -> Engine {
    let compiled = km_script::compile(source, "<test>", None).expect("test layout compiles");
    let keyboard = Keyboard::from_raw(compiled.keyboard).expect("test layout decodes");
    Engine::new(Arc::new(keyboard))
}

/// Types text one character at a time.
fn type_text(engine: &mut Engine, text: &str) {
    for char in text.chars() {
        engine.key(KeyInput::character(char));
    }
}

#[test]
fn a_literal_rule_replaces_what_was_typed() {
    let mut engine = engine("\"ka\" => U1000\n");
    type_text(&mut engine, "ka");
    assert_eq!(engine.context(), "\u{1000}");
}

#[test]
fn the_longest_pattern_wins_over_priority_order() {
    let mut engine = engine("\"ab\" => U1000\n\"b\" => U1001\n");
    type_text(&mut engine, "ab");
    assert_eq!(engine.context(), "\u{1000}", "`ab` beats `b`");
}

#[test]
fn anyof_captures_one_character_and_emits_it() {
    let mut engine = engine("$cons = U1000 + U1001\n$cons[*] => $1\n");
    type_text(&mut engine, "a");
    assert_eq!(engine.context(), "a", "the capture is the matched text");
}

#[test]
fn anyof_indexes_its_matching_row() {
    let mut engine = engine("$row1K = \"qw\"\n$row1U = U1040 + U1041\n$row1K[*] => $row1U[$1]\n");
    type_text(&mut engine, "q");
    assert_eq!(engine.context(), "\u{1040}");
    type_text(&mut engine, "w");
    assert_eq!(engine.context(), "\u{1040}\u{1041}");
}

#[test]
fn notanyof_matches_outside_the_set() {
    let mut engine = engine("$vowels = \"ae\"\n$vowels[^] => \"z\"\n");
    type_text(&mut engine, "k");
    assert_eq!(engine.context(), "z");
    type_text(&mut engine, "a");
    assert_eq!(
        engine.context(),
        "za",
        "a vowel matches no rule, so it passes through"
    );
}

#[test]
fn any_takes_one_printable_character() {
    let mut engine = engine("ANY => $1\n");
    let outcome = engine.key(KeyInput::character('k'));
    assert!(outcome.consumed);
    assert_eq!(engine.context(), "k");
    let space = engine.key(KeyInput::character(' '));
    assert!(!space.consumed, "ANY never matches a space");
}

#[test]
fn a_switch_lives_for_one_key_event() {
    let mut engine = engine("\"9\" => ('shape')\n('shape') + \"1\" => \"one\"\n");
    type_text(&mut engine, "9");
    type_text(&mut engine, "1");
    assert_eq!(engine.context(), "one");
    type_text(&mut engine, "1");
    assert_eq!(
        engine.context(),
        "one1",
        "the switch is spent after one key"
    );
}

#[test]
fn a_switch_can_re_arm_itself() {
    let mut engine = engine("\"9\" => ('shape')\n('shape') + ANY => $1 + ('shape')\n");
    type_text(&mut engine, "9ab");
    assert_eq!(engine.context(), "ab");
    assert!(!engine.switches().is_empty(), "a state re-armed itself");
}

#[test]
fn output_is_matched_again_until_it_is_one_ascii_character() {
    let mut engine = engine("\"x\" => U1000\nU1000 => U1001\nU1001 => \"y\"\n");
    type_text(&mut engine, "x");
    assert_eq!(engine.context(), "y", "two rounds of recursion ran");
}

#[test]
fn a_cycle_of_rules_stops_at_the_recursion_limit() {
    let mut engine = engine("\"a\" => U1000\nU1000 => U1001\nU1001 => U1000\n");
    let outcome = engine.key(KeyInput::character('a'));
    assert!(outcome.recursion_limit);
    assert!(
        !outcome.consumed,
        "a runaway rule reports failure, as upstream does"
    );
}

#[test]
fn virtual_keys_match_the_pressed_key_and_modifiers() {
    let mut engine = engine("<VK_SHIFT & VK_KEY_A> => \"A\"\n\"a\" => \"a\"\n");
    let plain = engine.key(KeyInput::character('a'));
    assert_eq!(plain.edit.insert, "a");
    let shifted = engine.key(KeyInput::character('A'));
    assert_eq!(shifted.edit.insert, "A");
}

#[test]
fn altgr_is_right_alt_and_ctrl_alt_when_the_layout_says_so() {
    let mut engine =
        engine("<VK_BACKSLASH & VK_ALT_GR> => U104B\n<VK_KEY_1 & VK_ALT_GR> => \"!\"\n");
    let alt_gr = KeyInput::virtual_key(0x31).with_modifiers(Modifiers::NONE.alt_gr());
    assert!(engine.key(alt_gr).consumed, "a host that knows about AltGr");
    let ctrl_alt = KeyInput::virtual_key(0x31).with_modifiers(Modifiers::NONE.control().alt());
    assert!(
        engine.key(ctrl_alt).consumed,
        "ctrl+alt stands in for AltGr"
    );
    let alt_only = KeyInput::virtual_key(0x31).with_modifiers(Modifiers::NONE.alt());
    assert!(
        !engine.key(alt_only).consumed,
        "left alt alone is not AltGr"
    );
}

#[test]
fn a_backspace_rule_can_swallow_the_key() {
    let mut engine = engine("<VK_BACK> => NULL\n");
    let outcome = engine.key(KeyInput::backspace());
    assert!(outcome.consumed);
    assert!(outcome.edit.is_empty());
}

#[test]
fn smart_backspace_restores_the_previous_context() {
    let mut engine = engine("/*\n@SMART_BACKSPACE = \"TRUE\"\n*/\n\"a\" => U1000\n");
    type_text(&mut engine, "a");
    assert_eq!(engine.context(), "\u{1000}");
    let outcome = engine.key(KeyInput::backspace());
    assert!(outcome.consumed);
    assert_eq!(engine.context(), "", "one step back, not one character");
    assert_eq!(outcome.edit.erase, 1);
}

#[test]
fn plain_backspace_deletes_one_character() {
    let mut engine = engine("\"a\" => U1000\n");
    type_text(&mut engine, "ab");
    let outcome = engine.key(KeyInput::backspace());
    assert!(outcome.consumed);
    assert_eq!(engine.context(), "\u{1000}");
    assert_eq!(outcome.edit.erase, 1);
}

#[test]
fn an_unmatched_character_enters_the_context() {
    let mut engine = engine("\"a\" => U1000\n");
    let outcome = engine.key(KeyInput::character('q'));
    assert!(outcome.consumed);
    assert_eq!(outcome.edit.insert, "q");
    assert_eq!(
        engine.context(),
        "q",
        "the engine tracks what the host now shows"
    );
}

#[test]
fn eat_swallows_unmatched_keys() {
    let mut engine = engine("/*\n@EAT_ALL_UNUSED_KEYS = \"TRUE\"\n*/\n\"a\" => U1000\n");
    let outcome = engine.key(KeyInput::character('q'));
    assert!(outcome.consumed);
    assert!(outcome.edit.is_empty());
}

#[test]
fn control_and_alt_leave_shortcuts_alone() {
    let mut engine = engine("ANY => $1\n");
    for modifiers in [Modifiers::NONE.control(), Modifiers::NONE.alt()] {
        let outcome = engine.key(KeyInput::character('c').with_modifiers(modifiers));
        assert!(!outcome.consumed);
    }
}

#[test]
fn a_character_without_a_key_code_is_still_typed() {
    let mut engine = engine("\"a\" => U1000\n");
    let outcome = engine.key(KeyInput::character('\u{1000}'));
    assert!(outcome.consumed);
    assert_eq!(engine.context(), "\u{1000}");
}

#[test]
fn reset_forgets_everything() {
    let mut engine = engine("\"9\" => ('shape')\n('shape') + \"1\" => \"one\"\n");
    type_text(&mut engine, "9");
    engine.reset();
    assert_eq!(engine.context(), "");
    assert_eq!(engine.switches(), []);
    type_text(&mut engine, "1");
    assert_eq!(engine.context(), "1");
}

#[test]
fn preview_leaves_the_context_alone() {
    let engine = engine("\"a\" => U1000\n");
    let preview = engine.preview(KeyInput::character('a'));
    assert_eq!(preview.edit.insert, "\u{1000}");
    assert_eq!(engine.context(), "");
}

#[test]
fn the_real_zawgyi_layout_loads_and_types() {
    let keyboard = Keyboard::from_bytes(include_bytes!("../../../fixtures/zawgyi-l-unicode.km2"))
        .expect("real keyboard loads");
    assert_eq!(keyboard.meta.name, "ZawgyiL Uni");
    assert_eq!(keyboard.rule_count(), 232);
    let mut engine = Engine::new(Arc::new(keyboard));
    let outcome = engine.key(KeyInput::character('1'));
    assert!(outcome.consumed);
    assert_eq!(
        engine.context(),
        "\u{1041}",
        "VK_KEY_1 maps to the digit one"
    );
}

#[test]
fn odd_files_never_panic() {
    let basic = include_bytes!("../../../fixtures/basic.km2");
    for length in 0..basic.len() {
        let _ = Keyboard::from_bytes(&basic[..length]);
    }
    for byte in 0..=255u8 {
        let mut corrupted = basic.to_vec();
        corrupted[18] = byte;
        if let Ok(keyboard) = Keyboard::from_bytes(&corrupted) {
            let mut engine = Engine::new(Arc::new(keyboard));
            type_text(&mut engine, "kashape");
            let _ = engine.key(KeyInput::backspace());
        }
    }
}
