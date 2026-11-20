//! The parsed shape of a layout script.

/// One piece of a variable's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    /// A `Uxxxx` point.
    Unicode(char),
    /// A quoted string.
    Text(String),
    /// Another variable, by name.
    Variable(String),
}

/// `$name = ...`, which also creates switches when used with `('name')`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDecl {
    /// Name without the leading `$`.
    pub name: String,
    /// Concatenated terms.
    pub value: Vec<Term>,
    /// 1-based source line.
    pub line: u32,
    /// 1-based source column.
    pub column: u32,
}

/// What a `[...]` after a variable asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    /// No brackets at all.
    None,
    /// `[*]`: one character from the variable.
    AnyOf,
    /// `[^]`: one character not in the variable.
    NotAnyOf,
    /// `[$2]`: the output at the position the second segment matched.
    Index(u16),
}

/// A left-side segment: what the rule waits for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lhs {
    /// Literal text.
    Text(String),
    /// A variable, optionally modified.
    Variable {
        /// Name without the leading `$`.
        name: String,
        /// The bracket modifier.
        modifier: Modifier,
    },
    /// `('name')`, true while the switch is on.
    Switch(String),
    /// `ANY`.
    Any,
    /// A predefined key used as text, which is how `NULL` appears.
    Key(String),
    /// `<VK_SHIFT & VK_KEY_A>`, a virtual-key combination.
    Keys(Vec<String>),
}

/// A right-side segment: what the rule produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rhs {
    /// Literal text.
    Text(String),
    /// A variable, optionally indexed by a capture.
    Variable {
        /// Name without the leading `$`.
        name: String,
        /// The `[$n]` index.
        index: Option<u16>,
    },
    /// `$1`: the text the first segment matched.
    Reference(u16),
    /// A predefined key, emitted as the character it names.
    Key(String),
    /// `('name')`, turning a switch on or off.
    Switch(String),
}

/// One `lhs => rhs` rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// What must match, in order.
    pub lhs: Vec<Lhs>,
    /// What is produced, in order.
    pub rhs: Vec<Rhs>,
    /// 1-based source line.
    pub line: u32,
    /// 1-based source column.
    pub column: u32,
}

/// A statement at the top level of a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    /// A variable definition.
    Variable(VariableDecl),
    /// A rule.
    Rule(Rule),
    /// An `@` option found in a comment.
    Option {
        /// Upper-cased option name.
        name: String,
        /// Raw value as written.
        value: String,
        /// 1-based source line.
        line: u32,
        /// 1-based source column.
        column: u32,
    },
    /// `include("file.kms")`.
    Include {
        /// Path as written, relative to the including file.
        path: String,
        /// 1-based source line.
        line: u32,
        /// 1-based source column.
        column: u32,
    },
}

/// Everything one source file declares, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Program {
    /// Statements in source order.
    pub statements: Vec<Statement>,
}
