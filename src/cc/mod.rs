//! Tiny C JIT compiler.
//!
//! Internal layout:
//!   lexer   — source text → tokens
//!   parser  — tokens → AST / values
//!   codegen — values → x86-64 machine bytes

pub mod lexer;
pub mod parser;
pub mod codegen;