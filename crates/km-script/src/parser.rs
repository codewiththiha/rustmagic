//! Building a [`Program`] from tokens.

use km_core::{Error, Result};

use crate::ast::{Lhs, Modifier, Program, Rhs, Rule, Statement, Term, VariableDecl};
use crate::token::{Spanned, Token};

/// Parses a token stream into statements.
///
/// Statements end at a newline; `\` continues a line, which the lexer has
/// already resolved. A stray token, a missing `=>` or an unclosed bracket is an
/// error that names where it happened.
pub fn parse(tokens: &[Spanned], path: &str) -> Result<Program> {
    let mut parser = Parser {
        tokens,
        pos: 0,
        path: path.to_owned(),
    };
    parser.program()
}

struct Parser<'a> {
    tokens: &'a [Spanned],
    pos: usize,
    path: String,
}

impl Parser<'_> {
    fn program(&mut self) -> Result<Program> {
        let mut program = Program::default();
        loop {
            while self.peek() == &Token::Newline {
                self.pos += 1;
            }
            let current = self.current().clone();
            match current.token {
                Token::Eof => return Ok(program),
                Token::Option { name, value } => {
                    program.statements.push(Statement::Option {
                        name,
                        value,
                        line: current.line,
                        column: current.column,
                    });
                    self.pos += 1;
                }
                Token::Include(path) => {
                    program.statements.push(Statement::Include {
                        path,
                        line: current.line,
                        column: current.column,
                    });
                    self.pos += 1;
                }
                Token::Ident(name) if self.peek_at(1) == &Token::Equals => {
                    self.pos += 2;
                    let value = self.terms()?;
                    program.statements.push(Statement::Variable(VariableDecl {
                        name,
                        value,
                        line: current.line,
                        column: current.column,
                    }));
                }
                _ => {
                    let rule = self.rule()?;
                    program.statements.push(Statement::Rule(rule));
                }
            }
            self.expect_end_of_statement()?;
        }
    }

    /// A variable's value: terms joined by `+`.
    fn terms(&mut self) -> Result<Vec<Term>> {
        let mut terms = Vec::new();
        loop {
            let term = match self.current().clone().token {
                Token::Unicode(point) => Term::Unicode(point),
                Token::Text(text) => Term::Text(text),
                Token::Ident(name) => Term::Variable(name),
                other => {
                    return Err(self.error_at_current(format!(
                        "expected a value, found {}",
                        other.describe()
                    )));
                }
            };
            terms.push(term);
            self.pos += 1;
            if self.peek() != &Token::Plus {
                return Ok(terms);
            }
            self.pos += 1;
        }
    }

    fn rule(&mut self) -> Result<Rule> {
        let start = self.current().clone();
        let lhs = self.lhs()?;
        if self.peek() != &Token::Arrow {
            return Err(self.error_at_current("a rule needs `=>` after its left side"));
        }
        self.pos += 1;
        let rhs = self.rhs()?;
        Ok(Rule {
            lhs,
            rhs,
            line: start.line,
            column: start.column,
        })
    }

    fn lhs(&mut self) -> Result<Vec<Lhs>> {
        let mut items = Vec::new();
        loop {
            items.push(self.lhs_item()?);
            if self.peek() != &Token::Plus {
                return Ok(items);
            }
            self.pos += 1;
        }
    }

    fn lhs_item(&mut self) -> Result<Lhs> {
        let current = self.current().clone();
        let item = match current.token {
            Token::Text(text) => Lhs::Text(text),
            Token::Unicode(point) => Lhs::Text(point.to_string()),
            Token::Any => Lhs::Any,
            Token::Predefined(name) => Lhs::Key(name),
            Token::Ident(name) => {
                self.pos += 1;
                let modifier = self.modifier(true)?;
                return Ok(Lhs::Variable { name, modifier });
            }
            Token::SwitchBegin => {
                self.pos += 1;
                let name = self.switch_name()?;
                return Ok(Lhs::Switch(name));
            }
            Token::VkBegin => {
                self.pos += 1;
                let keys = self.keys()?;
                return Ok(Lhs::Keys(keys));
            }
            other => {
                return Err(self.error_at_current(format!(
                    "expected a left-side segment, found {}",
                    other.describe()
                )));
            }
        };
        self.pos += 1;
        Ok(item)
    }

    fn rhs(&mut self) -> Result<Vec<Rhs>> {
        let mut items = Vec::new();
        loop {
            items.push(self.rhs_item()?);
            if self.peek() != &Token::Plus {
                return Ok(items);
            }
            self.pos += 1;
        }
    }

    fn rhs_item(&mut self) -> Result<Rhs> {
        let current = self.current().clone();
        let item = match current.token {
            Token::Text(text) => Rhs::Text(text),
            Token::Unicode(point) => Rhs::Text(point.to_string()),
            Token::Reference(index) => Rhs::Reference(index),
            Token::Predefined(name) => Rhs::Key(name),
            Token::Ident(name) => {
                self.pos += 1;
                let index = match self.modifier(false)? {
                    Modifier::Index(index) => Some(index),
                    Modifier::None => None,
                    Modifier::AnyOf | Modifier::NotAnyOf => {
                        return Err(self.error_at_current(
                            "`[*]` and `[^]` match text; they belong on the left side",
                        ));
                    }
                };
                return Ok(Rhs::Variable { name, index });
            }
            Token::SwitchBegin => {
                self.pos += 1;
                let name = self.switch_name()?;
                return Ok(Rhs::Switch(name));
            }
            other => {
                return Err(self.error_at_current(format!(
                    "expected a right-side segment, found {}",
                    other.describe()
                )));
            }
        };
        self.pos += 1;
        Ok(item)
    }

    /// Reads `( 'name' )` after the opening bracket has been consumed.
    fn switch_name(&mut self) -> Result<String> {
        let name = match self.current().clone().token {
            Token::Text(text) => text,
            other => {
                return Err(self.error_at_current(format!(
                    "a switch name must be quoted, found {}",
                    other.describe()
                )));
            }
        };
        self.pos += 1;
        if self.peek() != &Token::SwitchEnd {
            return Err(self.error_at_current("a switch name needs a closing `)`"));
        }
        self.pos += 1;
        Ok(name)
    }

    /// Reads the key names inside `<` and `>`.
    fn keys(&mut self) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        loop {
            match self.current().clone().token {
                Token::Predefined(name) => keys.push(name),
                other => {
                    return Err(self.error_at_current(format!(
                        "expected a virtual key, found {}",
                        other.describe()
                    )));
                }
            }
            self.pos += 1;
            match self.peek() {
                Token::And => self.pos += 1,
                Token::VkEnd => {
                    self.pos += 1;
                    return Ok(keys);
                }
                _ => return Err(self.error_at_current("a key list needs `>` to end it")),
            }
        }
    }

    /// Reads an optional `[...]` after a variable name.
    fn modifier(&mut self, matching: bool) -> Result<Modifier> {
        if self.peek() != &Token::ModifierBegin {
            return Ok(Modifier::None);
        }
        self.pos += 1;
        let modifier = match self.current().clone().token {
            Token::AnyOf => Modifier::AnyOf,
            Token::NotAnyOf => Modifier::NotAnyOf,
            Token::Reference(index) if !matching => Modifier::Index(index),
            other => {
                return Err(self.error_at_current(format!(
                    "`[` needs `*`, `^` or a capture index, found {}",
                    other.describe()
                )));
            }
        };
        self.pos += 1;
        if self.peek() != &Token::ModifierEnd {
            return Err(self.error_at_current("`[` needs `]` to close it"));
        }
        self.pos += 1;
        Ok(modifier)
    }

    fn expect_end_of_statement(&mut self) -> Result<()> {
        match self.peek() {
            Token::Newline => {
                self.pos += 1;
                Ok(())
            }
            Token::Eof | Token::Option { .. } => Ok(()),
            _ => Err(self.error_at_current("expected a new line after the statement")),
        }
    }

    fn current(&self) -> &Spanned {
        self.tokens.get(self.pos).unwrap_or_else(|| {
            self.tokens.last().unwrap_or(&Spanned {
                token: Token::Eof,
                line: 0,
                column: 0,
            })
        })
    }

    fn peek(&self) -> &Token {
        &self.current().token
    }

    fn peek_at(&self, offset: usize) -> &Token {
        self.tokens
            .get(self.pos + offset)
            .map_or(&Token::Eof, |spanned| &spanned.token)
    }

    fn error_at_current(&self, message: impl Into<String>) -> Error {
        let spanned = self.current();
        Error::script(&self.path, spanned.line, spanned.column, message)
    }
}
