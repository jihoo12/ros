use crate::std::stdio::{input, print};
use crate::std::syscall;

const MAX_CMD_LEN: usize = 64;
const HISTORY_SIZE: usize = 10;

struct Shell {
    history: [[u8; MAX_CMD_LEN]; HISTORY_SIZE],
    history_len: [usize; HISTORY_SIZE],
    history_count: usize, // Number of items in history
    history_start: usize, // Index of oldest item (ring buffer)
}

impl Shell {
    fn new() -> Self {
        Self {
            history: [[0; MAX_CMD_LEN]; HISTORY_SIZE],
            history_len: [0; HISTORY_SIZE],
            history_count: 0,
            history_start: 0,
        }
    }

    fn run(&mut self) {
        print("\n=== Interactive Shell (Fixed Buffer) ===\n");
        print("Type 'help' for commands. Use Arrow Keys for history.\n");

        loop {
            print("> ");
            let line = input();

            if line.is_empty() {
                continue;
            }

            // Save to history
            self.add_history(line.as_bytes());

            // Eval
            self.eval(&line);
        }
    }

    fn add_history(&mut self, cmd: &[u8]) {
        let idx = (self.history_start + self.history_count) % HISTORY_SIZE;

        // Copy to buffer
        let len = if cmd.len() > MAX_CMD_LEN {
            MAX_CMD_LEN
        } else {
            cmd.len()
        };
        self.history[idx][..len].copy_from_slice(&cmd[..len]);
        self.history_len[idx] = len;

        if self.history_count < HISTORY_SIZE {
            self.history_count += 1;
        } else {
            self.history_start = (self.history_start + 1) % HISTORY_SIZE;
        }
    }

    fn get_history(&self, offset_from_newest: usize) -> Option<&[u8]> {
        if offset_from_newest >= self.history_count {
            return None;
        }
        // newest is at start + count - 1
        // offset 0 => newest
        let end_idx = self.history_start + self.history_count;
        let target_idx = (end_idx - 1 - offset_from_newest) % HISTORY_SIZE;
        Some(&self.history[target_idx][..self.history_len[target_idx]])
    }

    fn eval(&self, line: &str) {
        let mut parts = line.split_whitespace();
        if let Some(cmd) = parts.next() {
            match cmd {
                "help" => {
                    print("Commands: help, echo, history, clear, shutdown\n");
                }
                "echo" => {
                    let mut first = true;
                    for arg in parts {
                        if !first {
                            print(" ");
                        }
                        print(arg);
                        first = false;
                    }
                    print("\n");
                }
                "history" => {
                    for i in 0..self.history_count {
                        if let Some(h) = self.get_history(self.history_count - 1 - i) {
                            // Print stored history oldest to newest
                            print(unsafe { str::from_utf8_unchecked(h) });
                            print("\n");
                        }
                    }
                }
                "shutdown" => {
                    print("Bye!\n");
                    unsafe { syscall(10, 0, 0, 0, 0, 0, 0) };
                }
                "clear" => {
                    unsafe { syscall(12, 0, 0, 0, 0, 0, 0) };
                }
                "asm" => {
                    use crate::tinyasm::encoder::{Instruction, Operand, encode_instruction};
                    use crate::tinyasm::jit::JitMemory;
                    use crate::tinyasm::registers::Register;

                    print("Running TinyASM Demo...\n");
                    //important!
                    //add Ret in end
                    let instrs = [
                        Instruction::Mov(Operand::Reg(Register::RAX), Operand::Imm64(10)),
                        Instruction::Syscall,
                        Instruction::Ret,
                    ];

                    let mut machine_code = alloc::vec::Vec::new();
                    print("Encoding...\n");
                    for inst in instrs.iter() {
                        if let Err(_) = encode_instruction(*inst, &mut machine_code) {
                            print("Encoding error\n");
                            return;
                        }
                    }
                    print("\nDone encoding.\n");
                    print("Encoded: ");
                    for _ in &machine_code {
                        // Simple hex print (manually since we don't have format! in print easily or it uses syscall)
                        // Actually, let's just print simple message.
                    }
                    print("bytes\n");

                    match JitMemory::new(4096) {
                        Ok(mut jit) => {
                            if let Err(_) = jit.write(&machine_code) {
                                print("JIT Write Error\n");
                            } else {
                                if let Err(_) = jit.make_executable() {
                                    print("JIT Make Executable Error\n");
                                } else {
                                    unsafe {
                                        let func = jit.as_fn_u64();
                                        let res = func();
                                        // We need to print res. shell doesn't have formatted print helpers accessible easily?
                                        // shell::print takes &str.
                                        // Let's use formatting if available or hack it.
                                        // We can use alloc::format! if we are in a crate with alloc.
                                        // shell.rs is part of main, which has extern crate alloc.
                                        // So we can use format! macro.
                                        let msg = alloc::format!("Result: {}\n", res);
                                        print(&msg);
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            print("JIT Alloc Error\n");
                        }
                    }
                }

                _ => {
                    print("Unknown command: ");
                    print(cmd);
                    print("\n");
                }
            }
        }
    }
}

pub fn shell() {
    let mut shell = Shell::new();
    shell.run();
}
