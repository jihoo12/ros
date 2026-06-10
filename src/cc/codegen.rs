use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::format;

use super::parser::{Function, Stmt, Expr};
use crate::tinyasm::parser::parse_asm_line;
use crate::tinyasm::encoder::assemble;

struct Relocation {
    target: String,
    patch_offset: usize,
}

pub fn compile_program(functions: &BTreeMap<String, Function>) -> Result<Vec<u8>, String> {
    let mut code = Vec::new();
    let mut relocs = Vec::new();
    let mut func_offsets = BTreeMap::new();

    // 1. Establish compilation order. 'main' must be compiled first so that it sits at offset 0.
    let mut compile_order = Vec::new();
    if let Some(main_func) = functions.get("main") {
        compile_order.push(main_func);
    } else {
        return Err("No 'main' function found".to_string());
    }

    for (name, func) in functions {
        if name != "main" {
            compile_order.push(func);
        }
    }

    // 2. Compile each function
    for func in compile_order {
        func_offsets.insert(func.name.clone(), code.len());

        // Prologue
        code.push(0x55); // push rbp
        code.push(0x48); // REX.W
        code.push(0x89);
        code.push(0xE5); // mov rbp, rsp

        // Pre-pass to count unique variable declarations and assign offsets from RBP
        let mut var_offsets = BTreeMap::new();
        let mut next_offset = 8;
        for stmt in &func.body {
            if let Stmt::VarDecl { name, .. } = stmt {
                if !var_offsets.contains_key(name) {
                    var_offsets.insert(name.clone(), next_offset);
                    next_offset += 8;
                }
            }
        }

        // Sub RSP to allocate stack space (aligned to 16 bytes)
        let stack_space = (next_offset - 8 + 15) & !15;
        if stack_space > 0 {
            // sub rsp, stack_space
            code.push(0x48); // REX.W
            code.push(0x81); // SUB
            code.push(0xEC); // ModR/M or opcode extension for RSP
            code.extend_from_slice(&(stack_space as u32).to_le_bytes());
        }

        // Compile statements
        for stmt in &func.body {
            match stmt {
                Stmt::VarDecl { name, val } => {
                    compile_expr(val, &mut code, &var_offsets, &mut relocs)?;
                    let offset = *var_offsets.get(name).unwrap();
                    let disp = -(offset as i32);
                    // mov [rbp - offset], rax
                    code.push(0x48);
                    code.push(0x89);
                    code.push(0x85);
                    code.extend_from_slice(&disp.to_le_bytes());
                }
                Stmt::Assign { name, val } => {
                    compile_expr(val, &mut code, &var_offsets, &mut relocs)?;
                    let offset = *var_offsets.get(name).ok_or_else(|| format!("Undeclared variable: {}", name))?;
                    let disp = -(offset as i32);
                    // mov [rbp - offset], rax
                    code.push(0x48);
                    code.push(0x89);
                    code.push(0x85);
                    code.extend_from_slice(&disp.to_le_bytes());
                }
                Stmt::Asm(asm_str) => {
                    let lines: Vec<_> = asm_str
                        .split(';')
                        .flat_map(|s| s.split('\n'))
                        .filter_map(|part| parse_asm_line(part.trim()))
                        .collect();
                    if !lines.is_empty() {
                        let asm_bytes = assemble(&lines).map_err(|e| format!("Asm error: {}", e))?;
                        code.extend_from_slice(&asm_bytes);
                    }
                }
                Stmt::Return(expr) => {
                    compile_expr(expr, &mut code, &var_offsets, &mut relocs)?;
                    code.push(0xC9); // leave
                    code.push(0xC3); // ret
                }
            }
        }

        // Default epilogue (just in case function has no return statement at the end)
        code.push(0xC9); // leave
        code.push(0xC3); // ret
    }

    // 3. Resolve relocations for function calls
    for rel in &relocs {
        let target_offset = *func_offsets.get(&rel.target).ok_or_else(|| format!("Undefined function: {}", rel.target))?;
        let next_instr = rel.patch_offset + 4;
        let rel_offset = (target_offset as isize) - (next_instr as isize);
        let rel_offset_i32 = rel_offset as i32;
        let bytes = rel_offset_i32.to_le_bytes();
        code[rel.patch_offset..rel.patch_offset + 4].copy_from_slice(&bytes);
    }

    Ok(code)
}

fn compile_expr(
    expr: &Expr,
    code: &mut Vec<u8>,
    var_offsets: &BTreeMap<String, usize>,
    relocs: &mut Vec<Relocation>,
) -> Result<(), String> {
    match expr {
        Expr::Number(n) => {
            // mov rax, n
            code.push(0x48);
            code.push(0xB8);
            code.extend_from_slice(&n.to_le_bytes());
        }
        Expr::Variable(name) => {
            let offset = var_offsets.get(name).ok_or_else(|| format!("Undeclared variable: {}", name))?;
            let disp = -(*offset as i32);
            // mov rax, [rbp - offset]
            code.push(0x48);
            code.push(0x8B);
            code.push(0x85);
            code.extend_from_slice(&disp.to_le_bytes());
        }
        Expr::Call(func_name) => {
            // call func_name
            code.push(0xE8);
            let patch_offset = code.len();
            code.extend_from_slice(&[0u8; 4]);
            relocs.push(Relocation {
                target: func_name.clone(),
                patch_offset,
            });
        }
    }
    Ok(())
}

/// Legacy single return value helper (kept for compatibility).
pub fn emit_return_u64(value: u64) -> Vec<u8> {
    let mut code = Vec::with_capacity(11);
    code.push(0x48);        // REX.W
    code.push(0xB8);        // MOV RAX, imm64
    code.extend_from_slice(&value.to_le_bytes());
    code.push(0xC3);        // RET
    code
}