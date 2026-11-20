//! The key-processing state machine.

use std::sync::Arc;

use km_core::{vk, KeyInput, Modifiers};

use crate::keyboard::Keyboard;
use crate::rule::{Emit, Pattern, Rule};

/// How many times one key event may re-match its own output before the engine
/// gives up, as upstream gives up at twenty.
pub const MAX_RECURSION: u32 = 20;

/// What a host must do to its text to show the engine's context.
///
/// KeyMagic only ever edits at the caret, so an edit is "remove `erase`
/// characters before the caret, then insert `insert`".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Edit {
    /// Characters to remove before the caret.
    pub erase: usize,
    /// Text to insert where they were.
    pub insert: String,
}

impl Edit {
    /// True when the edit changes nothing.
    pub fn is_empty(&self) -> bool {
        self.erase == 0 && self.insert.is_empty()
    }
}

/// What happened to one key event.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Whether the engine took the key; false means the host should type it.
    pub consumed: bool,
    /// How the host's text must change.
    pub edit: Edit,
    /// Source index of the rule that matched, for logs and the UI.
    pub rule: Option<usize>,
    /// True when the engine stopped itself at [`MAX_RECURSION`].
    pub recursion_limit: bool,
}

/// Everything that survives between key events.
#[derive(Debug, Clone, Default)]
struct State {
    context: String,
    history: Vec<String>,
    switches: Vec<(u16, bool)>,
    should_repeat: bool,
    recursion_limit: bool,
}

impl State {
    fn switch_on(&self, id: u16) -> bool {
        self.switches
            .iter()
            .find(|(switch, _)| *switch == id)
            .is_some_and(|(_, on)| *on)
    }

    fn toggle_switch(&mut self, id: u16) {
        match self.switches.iter_mut().find(|(switch, _)| *switch == id) {
            Some((_, on)) => *on = !*on,
            None => self.switches.push((id, true)),
        }
    }
}

/// One engine per input context.
///
/// A host creates an engine when its text field gains focus, feeds it every
/// key event, applies the [`Edit`] it returns, and calls [`Engine::reset`] when
/// focus is lost or the caret moves somewhere the engine cannot follow.
#[derive(Debug, Clone)]
pub struct Engine {
    keyboard: Arc<Keyboard>,
    state: State,
}

impl Engine {
    /// Builds an engine for a keyboard.
    pub fn new(keyboard: Arc<Keyboard>) -> Self {
        Self {
            keyboard,
            state: State::default(),
        }
    }

    /// The keyboard this engine runs.
    pub fn keyboard(&self) -> &Arc<Keyboard> {
        &self.keyboard
    }

    /// The text the engine believes is before the caret.
    pub fn context(&self) -> &str {
        &self.state.context
    }

    /// Replaces the context and keeps the history, for a host resynchronising.
    pub fn set_context(&mut self, context: &str) {
        self.state.context.clear();
        self.state.context.push_str(context);
    }

    /// Clears context, history, switches and the recursion flag.
    pub fn reset(&mut self) {
        self.state = State::default();
    }

    /// Which switches are on.
    pub fn switches(&self) -> &[(u16, bool)] {
        &self.state.switches
    }

    /// Processes one key event.
    ///
    /// Returns `consumed: false` when the host should type the key itself and
    /// leave the engine's context alone.
    pub fn key(&mut self, input: KeyInput) -> Outcome {
        let before = self.state.context.clone();
        self.state.recursion_limit = false;
        let mut matched_rule = None;
        let matched = if input.key > 0xFF && input.key != 0 {
            false
        } else {
            self.process_input(&input, input.text, &mut matched_rule)
        };
        if !matched && input.key >= 0x20 {
            self.state.switches.clear();
        }
        let consumed = if matched {
            self.repeat_output(&input, &before)
        } else {
            self.type_through(&input)
        };
        Outcome {
            consumed,
            edit: difference(&before, &self.state.context),
            rule: matched_rule,
            recursion_limit: self.state.recursion_limit,
        }
    }

    /// Runs the engine against a copy of its state, leaving it untouched.
    pub fn preview(&self, input: KeyInput) -> Outcome {
        let mut copy = self.clone();
        copy.key(input)
    }

    /// Tries every rule once, in match order.
    fn process_input(
        &mut self,
        input: &KeyInput,
        text: Option<char>,
        matched_rule: &mut Option<usize>,
    ) -> bool {
        let keyboard = Arc::clone(&self.keyboard);
        let tail: Vec<char> = self.state.context.chars().collect();
        for rule in &keyboard.rules {
            if !self.keys_match(rule, input) {
                continue;
            }
            let keyed = rule.key_count() > 0;
            let mut candidate = tail.clone();
            if !keyed {
                if let Some(char) = text {
                    candidate.push(char);
                }
            }
            if rule.match_len > candidate.len() {
                continue;
            }
            let start = candidate.len() - rule.match_len;
            let Some(captures) = match_patterns(rule, &candidate[start..], &self.state) else {
                continue;
            };
            let before_rule = self.state.context.clone();
            if input.key != 0 {
                self.state.switches.clear();
            }
            if !keyed {
                if let Some(char) = text {
                    self.state.context.push(char);
                }
            }
            let produced = self.apply_rule(rule, &captures);
            *matched_rule = Some(rule.index);
            return self.finish(rule, produced, &before_rule);
        }
        false
    }

    /// Whether the rule's key requirements hold for this event.
    ///
    /// Control and alt must be consumed by the rule: a layout never sees a
    /// shortcut. Shift is allowed to be held even when a rule does not ask for
    /// it, which is how upstream behaves.
    fn keys_match(&self, rule: &Rule, input: &KeyInput) -> bool {
        let requirement = &rule.keys;
        let modifiers = input.modifiers;
        if requirement.shift && !modifiers.shift {
            return false;
        }
        let mut control = modifiers.control;
        let mut alt = modifiers.alt;
        if requirement.control {
            if !control {
                return false;
            }
            control = false;
        }
        if requirement.alt {
            if !alt {
                return false;
            }
            alt = false;
        }
        if requirement.alt_gr && !modifiers.alt_gr {
            let through_ctrl_alt = self.keyboard.options.right_alt && alt && control;
            if through_ctrl_alt {
                alt = false;
                control = false;
            } else {
                return false;
            }
        }
        if control || alt {
            return false;
        }
        !requirement.keys.iter().any(|key| *key != input.key)
    }

    /// Turns captures and switches into the rule's output.
    ///
    /// `None` means a capture the rule asked for does not exist, which
    /// upstream treats as a failed match rather than a short output.
    fn apply_rule(&mut self, rule: &Rule, captures: &[String]) -> Option<String> {
        let mut out = String::new();
        for emit in &rule.emits {
            match emit {
                Emit::Text(text) => out.push_str(text),
                Emit::Reference(segment) => out.push_str(captures.get(*segment)?),
                Emit::Indexed {
                    table,
                    source,
                    segment,
                } => {
                    let capture = captures.get(*segment)?;
                    let Some(matched) = capture.chars().next() else {
                        continue;
                    };
                    if let Some(position) = source.chars().position(|char| char == matched) {
                        if let Some(replacement) = table.chars().nth(position) {
                            out.push(replacement);
                        }
                    }
                }
                Emit::Switch(id) => self.state.toggle_switch(*id),
            }
        }
        Some(out)
    }

    /// Replaces the matched tail and records whether to match again.
    ///
    /// A rule that leaves the context exactly as it found it also ends the
    /// chain: nothing about the context changed, so matching again would run
    /// the same rule forever. Upstream has no such guard and spends its twenty
    /// rounds before reporting failure, which makes `ANY => $1` swallow keys.
    fn finish(&mut self, rule: &Rule, produced: Option<String>, before: &str) -> bool {
        let Some(produced) = produced else {
            return false;
        };
        self.state.context = replace_tail(&self.state.context, rule.match_len, &produced);
        let progressed = self.state.context != before;
        self.state.should_repeat = !stops_recursion(&produced) && progressed;
        true
    }

    /// Feeds the engine its own output until it stops changing.
    fn repeat_output(&mut self, input: &KeyInput, before: &str) -> bool {
        let mut loops = 0;
        loop {
            if self.state.should_repeat {
                let mut ignored = None;
                let quiet = KeyInput {
                    key: 0,
                    text: None,
                    modifiers: Modifiers::NONE,
                };
                let matched = self.process_input(&quiet, None, &mut ignored);
                loops += 1;
                if loops >= MAX_RECURSION {
                    self.state.recursion_limit = true;
                    return false;
                }
                if input.key == vk::BACK && !self.state.history.is_empty() {
                    self.sync_history_after_backspace();
                    return true;
                }
                if !matched {
                    self.update_history(before);
                    return true;
                }
            } else if input.key == vk::BACK && !self.state.history.is_empty() {
                self.sync_history_after_backspace();
                return true;
            } else {
                self.update_history(before);
                return true;
            }
        }
    }

    /// The paths taken when no rule matched.
    fn type_through(&mut self, input: &KeyInput) -> bool {
        if input.modifiers.control || input.modifiers.alt {
            return false;
        }
        if input.key == vk::BACK {
            return self.backspace();
        }
        let Some(text) = input.text else {
            return false;
        };
        if input.key != 0 && input.key <= 0x20 {
            return false;
        }
        if self.keyboard.options.eat {
            return true;
        }
        self.state.context.push(text);
        true
    }

    fn backspace(&mut self) -> bool {
        if self.keyboard.options.auto_bksp && !self.state.history.is_empty() {
            if let Some(previous) = self.state.history.pop() {
                self.state.context = previous;
            }
            return true;
        }
        self.state.context.pop().is_some()
    }

    /// After a backspace inside the repeat loop: keep the history in step.
    fn sync_history_after_backspace(&mut self) {
        let same = self.state.history.last() == Some(&self.state.context);
        if same {
            self.state.history.pop();
        } else {
            self.state.history.clear();
        }
    }

    fn update_history(&mut self, before: &str) {
        if self.state.history.last().is_none_or(|last| last != before) {
            self.state.history.push(before.to_owned());
            if self.state.history.len() > HISTORY_LIMIT {
                self.state.history.remove(0);
            }
        }
    }
}

/// How many contexts smart backspace can walk back through.
const HISTORY_LIMIT: usize = 64;

/// Upstream's rule: output stops recursion when empty or one ASCII printable.
fn stops_recursion(produced: &str) -> bool {
    let mut chars = produced.chars();
    match (chars.next(), chars.next()) {
        (None, _) => true,
        (Some(char), None) => char > '\u{20}' && char < '\u{7F}',
        _ => false,
    }
}

/// Replaces the last `remove` characters of `context` with `insert`.
fn replace_tail(context: &str, remove: usize, insert: &str) -> String {
    let keep = context.chars().count().saturating_sub(remove);
    let mut out: String = context.chars().take(keep).collect();
    out.push_str(insert);
    out
}

/// The edit that turns `before` into `after`, editing only at the caret.
fn difference(before: &str, after: &str) -> Edit {
    let before_chars: Vec<char> = before.chars().collect();
    let after_chars: Vec<char> = after.chars().collect();
    let common = before_chars
        .iter()
        .zip(after_chars.iter())
        .take_while(|(left, right)| left == right)
        .count();
    Edit {
        erase: before_chars.len() - common,
        insert: after_chars[common..].iter().collect(),
    }
}

/// Matches a rule's patterns against exactly the tail they describe.
fn match_patterns(rule: &Rule, tail: &[char], state: &State) -> Option<Vec<String>> {
    let mut captures = Vec::new();
    let mut cursor = 0usize;
    for pattern in &rule.patterns {
        match pattern {
            Pattern::Text(expected) => {
                let length = expected.chars().count();
                let end = cursor.checked_add(length)?;
                let matched = tail.get(cursor..end)?;
                if !matched.iter().copied().eq(expected.chars()) {
                    return None;
                }
                captures.push(expected.clone());
                cursor = end;
            }
            Pattern::AnyOf(set) => {
                let char = *tail.get(cursor)?;
                if !set.contains(char) {
                    return None;
                }
                captures.push(char.to_string());
                cursor += 1;
            }
            Pattern::NotAnyOf(set) => {
                let char = *tail.get(cursor)?;
                if set.contains(char) {
                    return None;
                }
                captures.push(char.to_string());
                cursor += 1;
            }
            Pattern::Any => {
                let char = *tail.get(cursor)?;
                if !matches_any(char) {
                    return None;
                }
                captures.push(char.to_string());
                cursor += 1;
            }
            Pattern::Switch(id) => {
                if !state.switch_on(*id) {
                    return None;
                }
            }
            Pattern::Unsupported => return None,
        }
    }
    Some(captures)
}

/// Which characters `ANY` accepts: upstream takes printable ASCII but not
/// space, and the first block of Latin-1 and beyond.
pub fn matches_any(char: char) -> bool {
    let code = u32::from(char);
    (0x21..=0x7D).contains(&code) || (0xFF..0xFFFF).contains(&code)
}
