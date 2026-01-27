use super::BootInfo;
use core::fmt;
use font8x8::{UnicodeFonts, BASIC_FONTS};

pub struct Writer {
    framebuffer: *mut u8,
    info: BootInfo,
    x_pos: usize,
    y_pos: usize,
}

impl Writer {
    pub fn new(info: BootInfo) -> self::Writer {
        Self {
            framebuffer: info.framebuffer_base as *mut u8,
            info,
            x_pos: 0,
            y_pos: 0,
        }
    }

    pub fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.new_line(),
            c => {
                if self.x_pos >= self.info.horizontal_resolution as usize {
                    self.new_line();
                }
                if self.y_pos >= self.info.vertical_resolution as usize {
                    self.clear_screen(); // Simple scrolling: clear and reset. Better: scroll up.
                    self.y_pos = 0;
                }
                
                let bitmap = match BASIC_FONTS.get(c) {
                    Some(bitmap) => bitmap,
                    None => return, // Unknown char
                };

                self.write_rendered_char(bitmap);
                self.x_pos += 8;
            }
        }
    }

    fn write_rendered_char(&mut self, bitmap: [u8; 8]) {
        for (y, row) in bitmap.iter().enumerate() {
            for x in 0..8 {
                if match row {
                    &byte => byte >> x & 1 == 1,
                } {
                    self.write_pixel(self.x_pos + x, self.y_pos + y, 0xFFFFFFFF); // White
                } else {
                     self.write_pixel(self.x_pos + x, self.y_pos + y, 0x00000000); // Black background
                }
            }
        }
    }

    fn write_pixel(&mut self, x: usize, y: usize, color: u32) {
         if x >= self.info.horizontal_resolution as usize || y >= self.info.vertical_resolution as usize {
             return;
         }
         
         let pixel_offset = y * self.info.pixels_per_scanline as usize + x;
         // Assume 4 bytes per pixel (BGR or RGB Reserved) for typical UEFI GOP 32ppp
         // BootInfo.pixel_format should be checked, but we assume default for now.
         let ptr = self.framebuffer as *mut u32;
         unsafe {
             *ptr.add(pixel_offset) = color;
         }
    }

    fn new_line(&mut self) {
        self.x_pos = 0;
        self.y_pos += 16; // 8x8 font, strictly 8 high, but let's give some line spacing if we want. Let's stick to 8 or 10. 
        // font8x8 is 8x8.
        // Let's use 8 for now.
    }

    fn clear_screen(&mut self) {
         for y in 0..self.info.vertical_resolution {
             for x in 0..self.info.horizontal_resolution {
                 self.write_pixel(x as usize, y as usize, 0);
             }
         }
         self.x_pos = 0;
         self.y_pos = 0;
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}
