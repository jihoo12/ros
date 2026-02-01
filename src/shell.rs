use core::str;

// Keys mapped in xhci.rs
const KEY_RIGHT: u8 = 0x80;
const KEY_LEFT: u8 = 0x81;
const KEY_DOWN: u8 = 0x82;
const KEY_UP: u8 = 0x83;

// ASCII
const KEY_ENTER: u8 = b'\n';
const KEY_BACKSPACE: u8 = 0x08;

const MAX_CMD_LEN: usize = 64;
const HISTORY_SIZE: usize = 10;

fn user_print(s: &str) {
    unsafe {
        syscall(1, s.as_ptr() as usize, s.len(), 0, 0, 0, 0);
    }
}

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
        user_print("\n=== Interactive Shell (Fixed Buffer) ===\n");
        user_print("Type 'help' for commands. Use Arrow Keys for history.\n");

        loop {
            user_print("> ");
            let (len, buffer) = self.read_line();

            if len == 0 {
                continue;
            }

            // Save to history
            self.add_history(&buffer[..len]);

            // Eval
            if let Ok(cmd_str) = str::from_utf8(&buffer[..len]) {
                self.eval(cmd_str);
            } else {
                user_print("Error: Invalid UTF-8\n");
            }
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

    fn read_line(&mut self) -> (usize, [u8; MAX_CMD_LEN]) {
        let mut buffer = [0u8; MAX_CMD_LEN];
        let mut len = 0;
        let mut cursor = 0;
        let mut history_offset: isize = -1; // -1 means editing new line

        loop {
            // poll key
            unsafe { syscall(9, 0, 0, 0, 0, 0, 0) }; // Poll events

            let key_val = unsafe { syscall(11, 0, 0, 0, 0, 0, 0) };
            if key_val == 0 {
                continue;
            }
            let key = key_val as u8;

            match key {
                KEY_ENTER => {
                    user_print("\n");
                    break;
                }
                KEY_LEFT => {
                    if cursor > 0 {
                        cursor -= 1;
                        user_print("\x08");
                    }
                }
                KEY_RIGHT => {
                    if cursor < len {
                        // Print char at cursor to move right
                        let s = unsafe { str::from_utf8_unchecked(&buffer[cursor..cursor + 1]) };
                        user_print(s);
                        cursor += 1;
                    }
                }
                KEY_UP => {
                    if history_offset + 1 < self.history_count as isize {
                        history_offset += 1;
                        if let Some(hist) = self.get_history(history_offset as usize) {
                            self.replace_line(&mut buffer, &mut len, &mut cursor, hist);
                        }
                    }
                }
                KEY_DOWN => {
                    if history_offset > 0 {
                        history_offset -= 1;
                        if let Some(hist) = self.get_history(history_offset as usize) {
                            self.replace_line(&mut buffer, &mut len, &mut cursor, hist);
                        }
                    } else if history_offset == 0 {
                        history_offset = -1;
                        self.replace_line(&mut buffer, &mut len, &mut cursor, &[]);
                    }
                }
                KEY_BACKSPACE => {
                    if cursor > 0 {
                        // Delete at cursor-1
                        for i in (cursor - 1)..(len - 1) {
                            buffer[i] = buffer[i + 1];
                        }
                        len -= 1;
                        cursor -= 1;

                        user_print("\x08"); // move back
                        // print remainder
                        if cursor < len {
                            let s = unsafe { str::from_utf8_unchecked(&buffer[cursor..len]) };
                            user_print(s);
                        }
                        user_print(" "); // erase last
                        // move back to cursor
                        let chars_to_move_back = (len - cursor) + 1;
                        for _ in 0..chars_to_move_back {
                            user_print("\x08");
                        }
                    }
                }
                0x20..=0x7E => {
                    if len < MAX_CMD_LEN {
                        // Insert at cursor
                        for i in (cursor..len).rev() {
                            buffer[i + 1] = buffer[i];
                        }
                        buffer[cursor] = key;
                        len += 1;

                        // Print from cursor
                        let s = unsafe { str::from_utf8_unchecked(&buffer[cursor..len]) };
                        user_print(s);
                        cursor += 1;

                        // Move back
                        let chars_to_move_back = len - cursor;
                        for _ in 0..chars_to_move_back {
                            user_print("\x08");
                        }
                    }
                }
                _ => {}
            }
        }
        (len, buffer)
    }

    fn replace_line(
        &self,
        buffer: &mut [u8; MAX_CMD_LEN],
        len: &mut usize,
        cursor: &mut usize,
        new_content: &[u8],
    ) {
        // Clear current line from screen
        // Move cursor to start
        for _ in 0..*cursor {
            user_print("\x08");
        }
        // Print spaces
        for _ in 0..*len {
            user_print(" ");
        }
        // Move back
        for _ in 0..*len {
            user_print("\x08");
        }

        // Copy new content
        let new_len = if new_content.len() > MAX_CMD_LEN {
            MAX_CMD_LEN
        } else {
            new_content.len()
        };
        for i in 0..new_len {
            buffer[i] = new_content[i];
        }
        *len = new_len;
        *cursor = new_len;

        let s = unsafe { str::from_utf8_unchecked(&buffer[..*len]) };
        user_print(s);
    }

    fn eval(&self, line: &str) {
        let mut parts = line.split_whitespace();
        if let Some(cmd) = parts.next() {
            match cmd {
                "help" => {
                    user_print("Commands: help, echo, history, clear, shutdown\n");
                }
                "echo" => {
                    let mut first = true;
                    for arg in parts {
                        if !first {
                            user_print(" ");
                        }
                        user_print(arg);
                        first = false;
                    }
                    user_print("\n");
                }
                "history" => {
                    for i in 0..self.history_count {
                        if let Some(h) = self.get_history(self.history_count - 1 - i) {
                            // Print stored history oldest to newest
                            user_print(unsafe { str::from_utf8_unchecked(h) });
                            user_print("\n");
                        }
                    }
                }
                "shutdown" => {
                    user_print("Bye!\n");
                    unsafe { syscall(10, 0, 0, 0, 0, 0, 0) };
                }
                "clear" => {
                    unsafe { syscall(12, 0, 0, 0, 0, 0, 0) };
                }
                _ => {
                    user_print("Unknown command: ");
                    user_print(cmd);
                    user_print("\n");
                }
            }
        }
    }
}

pub fn shell() {
    let mut shell = Shell::new();
    shell.run();
}

#[inline(always)]
unsafe fn syscall(
    id: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> usize {
    let ret: usize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("r10") arg4,
            in("r8") arg5,
            in("r9") arg6,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, preserves_flags)
        );
    }
    ret
}
