//! Compiling a KeyMagic layout script (`.kms`) into a `.km2` keyboard.
//!
//! The pipeline is [`lexer`] then [`parser`] then [`compiler`]. Every step
//! reports the source line and column it failed at, and every step is total
//! over arbitrary input: a malformed file is an error, never a panic.
//!
//! Byte-for-byte agreement with the upstream compiler is a test, not a promise
//! of the type system; see `docs/porting.md`.

#![deny(missing_docs)]

mod ast;
mod compiler;
mod lexer;
mod parser;
mod token;

pub use ast::{Lhs, Modifier, Program, Rhs, Rule, Statement, Term, VariableDecl};
pub use compiler::{Compiled, Diagnostic, Stats, compile, compile_file};
pub use lexer::tokenize;
pub use parser::parse;
pub use token::{Spanned, Token};
