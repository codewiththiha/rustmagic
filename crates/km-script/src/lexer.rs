//! Turning `.kms` source into tokens.

use std::collections::VecDeque;

use km_core::{Error, Result};

use crate::token::{Spanned, Token};

/// Number of unicode escape digits accepted after `u` inside a string.
const UNICODE_ESCAPES: usize = 4;

/// Tokenises one source file.
///
/// Comments carry `@` options and are the only place options belong; an
/// unterminated string, a stray character or a bad escape is an error that
/// names the line and column.
pub fn tokenize(source: &str, path: &str) -> Result<Vec<Spanned>> {
    let mut lexer = Lexer {
        chars: source.chars().collect(),
        pos: 0,
        line: 1,
        column: 1,
        path: path.to_owned(),
        pending: VecDeque::new(),
    };
    let mut tokens = Vec::new();
    loop {
        let spanned = lexer.next_token()?;
        let done = spanned.token == Token::Eof;
        tokens.push(spanned);
        if done {
            return Ok(tokens);
        }
    }
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: u32,
    column: u32,
    path: String,
    pending: VecDeque<Spanned>,
}

impl Lexer {
    fn next_token(&mut self) -> Result<Spanned> {
        if let Some(spanned) = self.pending.pop_front() {
            return Ok(spanned);
        }
        loop {
            match self.peek() {
                None => return Ok(self.spanned(Token::Eof)),
                Some(' ' | '\t' | '\r') => {
                    self.bump();
                }
                Some('\\') => self.continuation()?,
                Some('\n') => {
                    let line = self.line;
                    let column = self.column;
                    self.bump();
                    return Ok(self.spanned_at(Token::Newline, line, column));
                }
                _ => break,
            }
        }
        let line = self.line;
        let column = self.column;
        let token = match self.peek().unwrap_or('\0') {
            '/' => self.comment_or_slash()?,
            '"' | '\'' => Token::Text(self.string()?),
            '@' => self.option()?,
            '$' => self.dollar()?,
            '=' => {
                self.bump();
                if self.peek() == Some('>') {
                    self.bump();
                    Token::Arrow
                } else {
                    Token::Equals
                }
            }
            '+' => self.single(Token::Plus),
            '&' => self.single(Token::And),
            '<' => self.single(Token::VkBegin),
            '>' => self.single(Token::VkEnd),
            '(' => self.single(Token::SwitchBegin),
            ')' => self.single(Token::SwitchEnd),
            '[' => self.single(Token::ModifierBegin),
            ']' => self.single(Token::ModifierEnd),
            '*' => self.single(Token::AnyOf),
            '^' => self.single(Token::NotAnyOf),
            'u' | 'U' if self.unicode_escape_ahead() => {
                self.bump();
                Token::Unicode(self.hex_char(UNICODE_ESCAPES)?)
            }
            'u' | 'U' | 'a'..='z' | 'A'..='Z' | '_' => self.word()?,
            other => {
                return Err(Error::script(
                    &self.path,
                    line,
                    column,
                    format!("unexpected character `{other}`"),
                ));
            }
        };
        Ok(self.spanned_at(token, line, column))
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let char = self.peek()?;
        self.pos += 1;
        if char == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(char)
    }

    fn spanned(&self, token: Token) -> Spanned {
        self.spanned_at(token, self.line, self.column)
    }

    fn spanned_at(&self, token: Token, line: u32, column: u32) -> Spanned {
        Spanned {
            token,
            line,
            column,
        }
    }

    fn single(&mut self, token: Token) -> Token {
        self.bump();
        token
    }

    fn error(&self, line: u32, column: u32, message: impl Into<String>) -> Error {
        Error::script(&self.path, line, column, message)
    }

    /// A backslash continues the statement on the next line.
    fn continuation(&mut self) -> Result<()> {
        let line = self.line;
        let column = self.column;
        self.bump();
        while matches!(self.peek(), Some(' ' | '\t' | '\r')) {
            self.bump();
        }
        if self.peek() == Some('\n') {
            self.bump();
            Ok(())
        } else {
            Err(self.error(line, column, "a backslash must be followed by a new line"))
        }
    }

    /// Reads a comment, collecting any `@` options inside it.
    fn comment_or_slash(&mut self) -> Result<Token> {
        if self.peek_at(1) == Some('/') {
            self.bump();
            self.bump();
            return self.scan_comment(false);
        }
        if self.peek_at(1) == Some('*') {
            self.bump();
            self.bump();
            return self.scan_comment(true);
        }
        let line = self.line;
        let column = self.column;
        Err(self.error(line, column, "`/` starts no comment"))
    }

    fn scan_comment(&mut self, block: bool) -> Result<Token> {
        loop {
            match self.peek() {
                None if block => {
                    return Err(self.error(self.line, self.column, "unterminated block comment"));
                }
                None => return Ok(Token::Newline),
                Some('@') => {
                    let line = self.line;
                    let column = self.column;
                    let token = self.option()?;
                    self.pending.push_back(self.spanned_at(token, line, column));
                }
                Some('*') if block && self.peek_at(1) == Some('/') => {
                    self.bump();
                    self.bump();
                    return self.skip_after_comment();
                }
                Some('\n') => {
                    if !block {
                        return Ok(Token::Newline);
                    }
                    self.bump();
                }
                Some(_) => {
                    self.bump();
                }
            }
        }
    }

    /// After a block comment, keep scanning what follows on this line.
    fn skip_after_comment(&mut self) -> Result<Token> {
        while matches!(self.peek(), Some(' ' | '\t' | '\r')) {
            self.bump();
        }
        match self.peek() {
            Some('\n') | None => Ok(Token::Newline),
            _ => self.next_token().map(|spanned| spanned.token),
        }
    }

    /// Reads `@NAME = "value"` inside a comment.
    fn option(&mut self) -> Result<Token> {
        let line = self.line;
        let column = self.column;
        self.bump();
        let mut name = String::new();
        while let Some(char) = self.peek() {
            if char.is_ascii_alphanumeric() || char == '_' || char == '-' {
                name.push(char.to_ascii_uppercase());
                self.bump();
            } else {
                break;
            }
        }
        if name.is_empty() {
            return Err(self.error(line, column, "`@` must be followed by an option name"));
        }
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
        let mut value = String::new();
        if self.peek() == Some('=') {
            self.bump();
            while matches!(self.peek(), Some(' ' | '\t')) {
                self.bump();
            }
            if matches!(self.peek(), Some('"' | '\'')) {
                value = self.string()?;
            }
        }
        Ok(Token::Option { name, value })
    }

    /// Reads `$name` or `$12`.
    fn dollar(&mut self) -> Result<Token> {
        let line = self.line;
        let column = self.column;
        self.bump();
        let mut word = String::new();
        while let Some(char) = self.peek() {
            if char.is_ascii_alphanumeric() || char == '_' {
                word.push(char);
                self.bump();
            } else {
                break;
            }
        }
        if word.is_empty() {
            return Err(self.error(line, column, "`$` must be followed by a name or number"));
        }
        if word.chars().all(|char| char.is_ascii_digit()) {
            let index = word.parse::<u16>().map_err(|_| {
                self.error(
                    line,
                    column,
                    format!("capture `$`{word} is larger than 65535"),
                )
            })?;
            return Ok(Token::Reference(index));
        }
        if word.starts_with(|char: char| char.is_ascii_digit()) {
            return Err(self.error(line, column, format!("`${word}` mixes digits and letters")));
        }
        Ok(Token::Ident(word))
    }

    /// Reads a quoted string, resolving escapes.
    fn string(&mut self) -> Result<String> {
        let line = self.line;
        let column = self.column;
        let quote = self.bump().unwrap_or('"');
        let mut text = String::new();
        loop {
            let Some(char) = self.peek() else {
                return Err(self.error(line, column, "unterminated string"));
            };
            if char == quote {
                self.bump();
                return Ok(text);
            }
            if char == '\n' {
                return Err(self.error(line, column, "a string cannot span lines"));
            }
            self.bump();
            if char != '\\' {
                text.push(char);
                continue;
            }
            let escape_line = self.line;
            let escape_column = self.column;
            let Some(escape) = self.bump() else {
                return Err(self.error(line, column, "unterminated escape"));
            };
            let resolved = match escape {
                'b' => '\u{8}',
                't' => '\t',
                'n' => '\n',
                'v' => '\u{b}',
                'f' => '\u{c}',
                'r' => '\r',
                'u' | 'U' => self.hex_char(4)?,
                'x' | 'X' => self.hex_char(2)?,
                '\\' | '"' | '\'' => escape,
                '\n' => continue,
                other => {
                    return Err(self.error(
                        escape_line,
                        escape_column,
                        format!("unknown escape `\\{other}`"),
                    ));
                }
            };
            text.push(resolved);
        }
    }

    /// Reads up to `max` hex digits, needing at least one, into a character.
    fn hex_char(&mut self, max: usize) -> Result<char> {
        let line = self.line;
        let column = self.column;
        let mut value = 0u32;
        let mut digits = 0;
        while digits < max {
            match self.peek().and_then(|char| char.to_digit(16)) {
                Some(digit) => {
                    value = value * 16 + digit;
                    digits += 1;
                    self.bump();
                }
                None => break,
            }
        }
        if digits == 0 {
            return Err(self.error(line, column, "expected hex digits after `u` or `x`"));
        }
        char::from_u32(value).ok_or_else(|| {
            self.error(
                line,
                column,
                format!("`{value:X}` is not a unicode scalar value"),
            )
        })
    }

    /// True when the current `u`/`U` starts a four-digit unicode point.
    fn unicode_escape_ahead(&self) -> bool {
        (1..=UNICODE_ESCAPES).all(|offset| {
            self.peek_at(offset)
                .is_some_and(|char| char.is_ascii_hexdigit())
        })
    }

    /// Reads a bare word: `ANY`, `include(...)` or a predefined key name.
    fn word(&mut self) -> Result<Token> {
        let line = self.line;
        let column = self.column;
        let mut word = String::new();
        while let Some(char) = self.peek() {
            if char.is_ascii_alphanumeric() || char == '_' {
                word.push(char);
                self.bump();
            } else {
                break;
            }
        }
        if word.eq_ignore_ascii_case("any") {
            return Ok(Token::Any);
        }
        if word.eq_ignore_ascii_case("include") {
            return self.include(line, column);
        }
        if km_core::is_predefined(&word) {
            return Ok(Token::Predefined(word));
        }
        Err(self.error(line, column, format!("`{word}` is not a known name")))
    }

    /// Reads the `("path")` part of an include.
    fn include(&mut self, line: u32, column: u32) -> Result<Token> {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
        if self.peek() != Some('(') {
            return Err(self.error(line, column, "include needs `(\"file.kms\")`"));
        }
        self.bump();
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
        if !matches!(self.peek(), Some('"' | '\'')) {
            return Err(self.error(line, column, "include needs a quoted file name"));
        }
        let path = self.string()?;
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
        if self.peek() != Some(')') {
            return Err(self.error(line, column, "include needs a closing `)`"));
        }
        self.bump();
        Ok(Token::Include(path))
    }
}
