use super::encoder::{Instruction, Operand};
use super::registers::Register;
use alloc::vec::Vec;

pub fn parse_register(s: &str) -> Option<Register> {
    match s.to_lowercase().as_str() {
        "rax" => Some(Register::RAX),
        "rcx" => Some(Register::RCX),
        "rdx" => Some(Register::RDX),
        "rbx" => Some(Register::RBX),
        "rsp" => Some(Register::RSP),
        "rbp" => Some(Register::RBP),
        "rsi" => Some(Register::RSI),
        "rdi" => Some(Register::RDI),
        "r8" => Some(Register::R8),
        "r9" => Some(Register::R9),
        "r10" => Some(Register::R10),
        "r11" => Some(Register::R11),
        "r12" => Some(Register::R12),
        "r13" => Some(Register::R13),
        "r14" => Some(Register::R14),
        "r15" => Some(Register::R15),
        _ => None,
    }
}

pub fn parse_operand(s: &str) -> Option<Operand> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // Try parsing as register
    if let Some(reg) = parse_register(s) {
        return Some(Operand::Reg(reg));
    }

    // Try parsing as immediate (hex or decimal)
    if s.starts_with("0x") {
        if let Ok(val) = u64::from_str_radix(&s[2..], 16) {
            if val <= 0xFFFF_FFFF {
                return Some(Operand::Imm32(val as i32));
            } else {
                return Some(Operand::Imm64(val));
            }
        }
    } else {
        if let Ok(val) = s.parse::<i32>() {
            return Some(Operand::Imm32(val));
        } else if let Ok(val) = s.parse::<u64>() {
            return Some(Operand::Imm64(val));
        }
    }

    // TODO: Support memory operands if needed
    None
}

pub fn parse_instruction(line: &str) -> Option<Instruction> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let mut parts = line.split_whitespace();
    let mnemonic = parts.next()?.to_lowercase();
    let rest = line[mnemonic.len()..].trim();

    match mnemonic.as_str() {
        "mov" | "add" | "sub" | "and" | "or" | "xor" | "cmp" | "shl" | "shr" => {
            let operands: Vec<&str> = rest.split(',').collect();
            if operands.len() == 2 {
                let dst = parse_operand(operands[0])?;
                let src = parse_operand(operands[1])?;
                match mnemonic.as_str() {
                    "mov" => Some(Instruction::Mov(dst, src)),
                    "add" => Some(Instruction::Add(dst, src)),
                    "sub" => Some(Instruction::Sub(dst, src)),
                    "and" => Some(Instruction::And(dst, src)),
                    "or" => Some(Instruction::Or(dst, src)),
                    "xor" => Some(Instruction::Xor(dst, src)),
                    "cmp" => Some(Instruction::Cmp(dst, src)),
                    "shl" => Some(Instruction::Shl(dst, src)),
                    "shr" => Some(Instruction::Shr(dst, src)),
                    _ => unreachable!(),
                }
            } else {
                None
            }
        }
        "mul" | "div" | "not" | "call" | "jmp" => {
            let op = parse_operand(rest)?;
            match mnemonic.as_str() {
                "mul" => Some(Instruction::Mul(op)),
                "div" => Some(Instruction::Div(op)),
                "not" => Some(Instruction::Not(op)),
                "call" => Some(Instruction::Call(op)),
                "jmp" => Some(Instruction::Jmp(op)),
                _ => unreachable!(),
            }
        }
        "syscall" => Some(Instruction::Syscall),
        "ret" => Some(Instruction::Ret),
        _ => None,
    }
}
