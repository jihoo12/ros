use crate::tinyasm::encoder::{Instruction, Operand};
use crate::tinyasm::registers::Register;
use alloc::vec::Vec;

pub fn parse_instruction(input: &str) -> Option<Instruction> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    let mut parts = input.splitn(2, |c: char| c.is_whitespace());
    let mnemonic = parts.next()?;
    let rest = parts.next().unwrap_or("").trim();

    if mnemonic.eq_ignore_ascii_case("syscall") {
        return Some(Instruction::Syscall);
    } else if mnemonic.eq_ignore_ascii_case("not") {
        let op = parse_operand(rest)?;
        return Some(Instruction::Not(op));
    } else if mnemonic.eq_ignore_ascii_case("mul") {
        let op = parse_operand(rest)?;
        return Some(Instruction::Mul(op));
    } else if mnemonic.eq_ignore_ascii_case("div") {
        let op = parse_operand(rest)?;
        return Some(Instruction::Div(op));
    } else if mnemonic.eq_ignore_ascii_case("mov")
        || mnemonic.eq_ignore_ascii_case("add")
        || mnemonic.eq_ignore_ascii_case("sub")
        || mnemonic.eq_ignore_ascii_case("and")
        || mnemonic.eq_ignore_ascii_case("or")
        || mnemonic.eq_ignore_ascii_case("xor")
        || mnemonic.eq_ignore_ascii_case("shl")
        || mnemonic.eq_ignore_ascii_case("shr")
    {
        let operands: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();
        if operands.len() != 2 {
            return None;
        }
        let dst = parse_operand(operands[0])?;
        let src = parse_operand(operands[1])?;

        if mnemonic.eq_ignore_ascii_case("mov") {
            return Some(Instruction::Mov(dst, src));
        }
        if mnemonic.eq_ignore_ascii_case("add") {
            return Some(Instruction::Add(dst, src));
        }
        if mnemonic.eq_ignore_ascii_case("sub") {
            return Some(Instruction::Sub(dst, src));
        }
        if mnemonic.eq_ignore_ascii_case("and") {
            return Some(Instruction::And(dst, src));
        }
        if mnemonic.eq_ignore_ascii_case("or") {
            return Some(Instruction::Or(dst, src));
        }
        if mnemonic.eq_ignore_ascii_case("xor") {
            return Some(Instruction::Xor(dst, src));
        }
        if mnemonic.eq_ignore_ascii_case("shl") {
            return Some(Instruction::Shl(dst, src));
        }
        if mnemonic.eq_ignore_ascii_case("shr") {
            return Some(Instruction::Shr(dst, src));
        }
    }

    None
}

fn parse_operand(input: &str) -> Option<Operand> {
    if input.is_empty() {
        return None;
    }

    // Try Register
    if let Some(reg) = parse_register(input) {
        return Some(Operand::Reg(reg));
    }

    // Try Immediate (hex or dec)
    if input.starts_with("0x") || input.starts_with("0X") {
        if let Ok(val) = u64::from_str_radix(&input[2..], 16) {
            return Some(Operand::Imm64(val));
        }
    } else if let Ok(val) = i64::from_str_radix(input, 10) {
        if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
            return Some(Operand::Imm32(val as i32));
        } else {
            return Some(Operand::Imm64(val as u64));
        }
    }

    None
}

fn parse_register(input: &str) -> Option<Register> {
    if input.eq_ignore_ascii_case("rax") {
        Some(Register::RAX)
    } else if input.eq_ignore_ascii_case("rcx") {
        Some(Register::RCX)
    } else if input.eq_ignore_ascii_case("rdx") {
        Some(Register::RDX)
    } else if input.eq_ignore_ascii_case("rbx") {
        Some(Register::RBX)
    } else if input.eq_ignore_ascii_case("rsp") {
        Some(Register::RSP)
    } else if input.eq_ignore_ascii_case("rbp") {
        Some(Register::RBP)
    } else if input.eq_ignore_ascii_case("rsi") {
        Some(Register::RSI)
    } else if input.eq_ignore_ascii_case("rdi") {
        Some(Register::RDI)
    } else if input.eq_ignore_ascii_case("r8") {
        Some(Register::R8)
    } else if input.eq_ignore_ascii_case("r9") {
        Some(Register::R9)
    } else if input.eq_ignore_ascii_case("r10") {
        Some(Register::R10)
    } else if input.eq_ignore_ascii_case("r11") {
        Some(Register::R11)
    } else if input.eq_ignore_ascii_case("r12") {
        Some(Register::R12)
    } else if input.eq_ignore_ascii_case("r13") {
        Some(Register::R13)
    } else if input.eq_ignore_ascii_case("r14") {
        Some(Register::R14)
    } else if input.eq_ignore_ascii_case("r15") {
        Some(Register::R15)
    } else {
        None
    }
}
