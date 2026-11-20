//! Tokens of the layout script language.

/// One token of a `.kms` source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// `$name`, a variable or switch name.
    Ident(String),
    /// `$12`, a capture reference.
    Reference(u16),
    /// `U1000`, one unicode point written as four hex digits.
    Unicode(char),
    /// A quoted string, with escapes already resolved.
    Text(String),
    /// `ANY`, the match-anything wildcard.
    Any,
    /// `VK_BACK`, `NULL` and the other predefined names.
    Predefined(String),
    /// `<`, opening a virtual-key state list.
    VkBegin,
    /// `>`, closing it.
    VkEnd,
    /// `(`, opening a switch name.
    SwitchBegin,
    /// `)`, closing it.
    SwitchEnd,
    /// `[`, opening a variable modifier.
    ModifierBegin,
    /// `]`, closing it.
    ModifierEnd,
    /// `*`, match one character of the variable.
    AnyOf,
    /// `^`, match one character not in the variable.
    NotAnyOf,
    /// `=>`, separates a rule's two sides.
    Arrow,
    /// `=`, separates a variable's name from its value.
    Equals,
    /// `+`, joins segments.
    Plus,
    /// `&`, joins virtual-key states.
    And,
    /// `include("file.kms")`, resolved before parsing.
    Include(String),
    /// `@NAME = "value"`, read out of a comment.
    Option {
        /// Option name as written.
        name: String,
        /// Option value, empty when the source gives none.
        value: String,
    },
    /// End of a line, which ends a statement.
    Newline,
    /// End of the token stream.
    Eof,
}

impl Token {
    /// How the token is named in an error message.
    pub fn describe(&self) -> String {
        match self {
            Self::Ident(name) => format!("variable `${name}`"),
            Self::Reference(index) => format!("reference `${index}`"),
            Self::Unicode(point) => format!("unicode `U{:04X}`", u32::from(*point)),
            Self::Text(text) => format!("text `{text}`"),
            Self::Any => "`ANY`".to_owned(),
            Self::Predefined(name) => format!("predefined key `{name}`"),
            Self::VkBegin => "`<`".to_owned(),
            Self::VkEnd => "`>`".to_owned(),
            Self::SwitchBegin => "`(`".to_owned(),
            Self::SwitchEnd => "`)`".to_owned(),
            Self::ModifierBegin => "`[`".to_owned(),
            Self::ModifierEnd => "`]`".to_owned(),
            Self::AnyOf => "`*`".to_owned(),
            Self::NotAnyOf => "`^`".to_owned(),
            Self::Arrow => "`=>`".to_owned(),
            Self::Equals => "`=`".to_owned(),
            Self::Plus => "`+`".to_owned(),
            Self::And => "`&`".to_owned(),
            Self::Include(path) => format!("include `{path}`"),
            Self::Option { name, .. } => format!("option `{name}`"),
            Self::Newline => "end of line".to_owned(),
            Self::Eof => "end of file".to_owned(),
        }
    }
}

/// A token together with the position that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned {
    /// The token.
    pub token: Token,
    /// 1-based line in the source.
    pub line: u32,
    /// 1-based column, counted in characters.
    pub column: u32,
}
