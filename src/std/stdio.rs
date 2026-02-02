use super::syscall;
use alloc::string::String;

// Keyboard constants mapped in xhci.rs
pub const KEY_RIGHT: u8 = 0x80;
pub const KEY_LEFT: u8 = 0x81;
pub const KEY_DOWN: u8 = 0x82;
pub const KEY_UP: u8 = 0x83;
pub const KEY_ENTER: u8 = b'\n';
pub const KEY_BACKSPACE: u8 = 0x08;

/// Prints a string to the standard output using the sys_print syscall.
pub fn print(s: &str) {
    unsafe {
        syscall(1, s.as_ptr() as usize, s.len(), 0, 0, 0, 0);
    }
}

/// Reads a line from the standard input by polling for keys.
/// Supports cursors, arrow keys, and insertion.
pub fn input() -> String {
    let mut s = String::new();
    let mut cursor = 0;

    loop {
        // Poll for events (sys_xhci_poll)
        unsafe {
            syscall(9, 0, 0, 0, 0, 0, 0);
        }

        // Read key (sys_read_key)
        let key_val = unsafe { syscall(11, 0, 0, 0, 0, 0, 0) };
        if key_val == 0 {
            continue;
        }
        let key = key_val as u8;

        match key {
            KEY_ENTER | 0x0D => {
                // Enter (\n or \r)
                print("\n");
                break;
            }
            KEY_LEFT => {
                if cursor > 0 {
                    cursor -= 1;
                    print("\x08");
                }
            }
            KEY_RIGHT => {
                if cursor < s.len() {
                    // Move right by re-printing the character at cursor
                    let c = s.as_bytes()[cursor];
                    let mut buf = [0u8; 4];
                    print((c as char).encode_utf8(&mut buf));
                    cursor += 1;
                }
            }
            KEY_BACKSPACE => {
                if cursor > 0 {
                    // Delete at cursor-1
                    s.remove(cursor - 1);
                    cursor -= 1;

                    print("\x08"); // move back
                    let suffix = &s[cursor..];
                    print(suffix); // print shifted remainder
                    print(" "); // erase last
                    // move back to cursor
                    for _ in 0..suffix.len() + 1 {
                        print("\x08");
                    }
                }
            }
            0x20..=0x7E => {
                // Printable ASCII
                if s.len() < 1024 {
                    // Reasonable limit
                    let c = key as char;
                    s.insert(cursor, c);

                    let suffix = &s[cursor..];
                    print(suffix);
                    cursor += 1;

                    // Move back to new cursor position
                    for _ in 0..suffix.len() - 1 {
                        print("\x08");
                    }
                }
            }
            _ => {}
        }
    }
    s
}
