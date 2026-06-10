/// A very simple C JIT compiler.
///
/// Supported grammar (a single function):
///
///   uint64_t <name>() { return <integer>; }
///
/// The compiler lexes the source, parses the return value, emits
/// x86-64 machine code that loads the constant into RAX and returns,
/// then executes it through `JitMemory`.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;

use crate::tinyasm::jit::JitMemory; // adjust path to wherever JitMemory lives

// ─── Lexer ────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
enum Token {
    Ident(String),
    Number(u64),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,
    Return,
}

fn lex(src: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = src.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => { chars.next(); }
            '(' => { tokens.push(Token::LParen);   chars.next(); }
            ')' => { tokens.push(Token::RParen);   chars.next(); }
            '{' => { tokens.push(Token::LBrace);   chars.next(); }
            '}' => { tokens.push(Token::RBrace);   chars.next(); }
            ';' => { tokens.push(Token::Semicolon);chars.next(); }
            '0'..='9' => {
                let mut num = String::new();
                while let Some(&d) = chars.peek() {
                    if d.is_ascii_digit() { num.push(d); chars.next(); } else { break; }
                }
                let n = num.parse::<u64>()
                    .map_err(|_| format!("Bad number: {}", num))?;
                tokens.push(Token::Number(n));
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' { word.push(c); chars.next(); } else { break; }
                }
                let tok = if word == "return" { Token::Return } else { Token::Ident(word) };
                tokens.push(tok);
            }
            other => return Err(format!("Unexpected char: '{}'", other)),
        }
    }
    Ok(tokens)
}

// ─── Parser ───────────────────────────────────────────────────────────────────

/// Parse `uint64_t <name>() { return <n>; }` and extract `n`.
fn parse_return_value(tokens: &[Token]) -> Result<u64, String> {
    let mut i = 0;

    // return-type identifier (e.g. `uint64_t` — we accept any ident)
    match tokens.get(i) {
        Some(Token::Ident(_)) => i += 1,
        _ => return Err("Expected return-type identifier".to_string()),
    }
    // function name
    match tokens.get(i) {
        Some(Token::Ident(_)) => i += 1,
        _ => return Err("Expected function name".to_string()),
    }
    // ()
    expect(tokens, &mut i, &Token::LParen)?;
    expect(tokens, &mut i, &Token::RParen)?;
    // {
    expect(tokens, &mut i, &Token::LBrace)?;
    // return <n> ;
    expect(tokens, &mut i, &Token::Return)?;
    let value = match tokens.get(i) {
        Some(Token::Number(n)) => { i += 1; *n }
        _ => return Err("Expected integer literal after 'return'".to_string()),
    };
    expect(tokens, &mut i, &Token::Semicolon)?;
    expect(tokens, &mut i, &Token::RBrace)?;

    Ok(value)
}

fn expect(tokens: &[Token], i: &mut usize, expected: &Token) -> Result<(), String> {
    match tokens.get(*i) {
        Some(t) if t == expected => { *i += 1; Ok(()) }
        other => Err(format!("Expected {:?}, got {:?}", expected, other)),
    }
}

// ─── Code generator ───────────────────────────────────────────────────────────

/// Emit x86-64 code for:
///   mov rax, <imm64>   ; 48 B8 <8-byte-le>
///   ret                ; C3
fn emit_return_u64(value: u64) -> Vec<u8> {
    let mut code = Vec::with_capacity(11);
    // REX.W + MOV RAX, imm64
    code.push(0x48);
    code.push(0xB8);
    code.extend_from_slice(&value.to_le_bytes());
    // RET
    code.push(0xC3);
    code
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Compile a tiny C function and return its result.
///
/// # Example
///
/// ```
/// let result = compile_and_run("uint64_t answer() { return 42; }").unwrap();
/// assert_eq!(result, 42);
/// ```
pub fn compile_and_run(src: &str) -> Result<u64, String> {
    // 1. Lex
    let tokens = lex(src)?;

    // 2. Parse
    let return_value = parse_return_value(&tokens)?;

    // 3. Emit machine code
    let code = emit_return_u64(return_value);

    // 4. Load into executable memory and call
    let mut mem = JitMemory::new(code.len())?;
    mem.write(&code)?;
    mem.make_executable()?;

    let result = unsafe { mem.as_fn_u64()() };
    Ok(result)
}