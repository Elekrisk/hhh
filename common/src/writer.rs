
use core::{fmt::{Arguments, Write}, sync::atomic::AtomicBool};

use crate::Framebuffer;

static FONT: &[[[u8; 8]; 16]; 128] = unsafe { core::mem::transmute(include_bytes!("../vgafont.bin")) };

#[used]
static mut WRITER: Writer = Writer { char_size: (0, 0), cursor: (0, 0), framebuffer: Framebuffer { ptr: 0 as _, resolution_x: 0, resolution_y: 0, stride: 0 } };

struct Writer {
    framebuffer: Framebuffer,
    cursor: (usize, usize),
    char_size: (usize, usize)
}

impl Writer {
    fn clear(&mut self) {
        for y in 0..self.framebuffer.resolution_y {
            unsafe {
                core::ptr::write_bytes(self.framebuffer.ptr.add(y * self.framebuffer.stride), 0, self.framebuffer.resolution_x);
            }
        }
    }

    fn write_char(&mut self, c: char) {
        let glyph = FONT[c as usize];
        let (cy, cx) = self.cursor;
        match c {
            '\n' => {
                self.cursor = (cy + 1, 0);
            },
            o if o < ' ' => {},
            _ => {
                for y in 0..16 {
                    for x in 0..8 {
                        let base = self.framebuffer.ptr;
                        let grayscale = glyph[y][x];
                        let color = ((grayscale as u32) << 16) | ((grayscale as u32) << 8) | grayscale as u32;
                        unsafe { base.add((cy * 16 + y) * self.framebuffer.stride + cx * 8 + x).write(color); }
                    }
                }
                self.cursor.1 += 1;
                if self.cursor.1 >= self.char_size.1 {
                   self.cursor.1 = 0;
                   self.cursor.0 += 1;
                }
            }
        }
        if self.cursor.0 >= self.char_size.0 {
            self.scroll();
        }
    }

    fn scroll(&mut self) {
        for y in 0..self.framebuffer.resolution_y - 16 {
            let base = self.framebuffer.ptr;
            let stride = self.framebuffer.stride;
            let res_x = self.framebuffer.resolution_x;
            for x in 0..res_x {
                unsafe {
                    base.add(y * stride + x).write_volatile(base.add((y + 16) * stride + x).read_volatile());
                }
            }
            // unsafe {
            //     core::ptr::copy(base.add((y + 16) * stride), base.add(y * stride), res_x);
            // }
        }
        for y in self.framebuffer.resolution_y-16..self.framebuffer.resolution_y {

            let base = self.framebuffer.ptr;
            let stride = self.framebuffer.stride;
            let res_x = self.framebuffer.resolution_x;
            unsafe {
                core::ptr::write_bytes(base.add(y * stride), 0, res_x);
            }
        }
        self.cursor.0 -= 1;
    }
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}

pub unsafe fn init(framebuffer: Framebuffer) {
    WRITER = Writer {
        char_size: (framebuffer.resolution_y / 16, framebuffer.resolution_x / 8),
        framebuffer,
        cursor: (0, 0),
    };
}

pub unsafe fn update_ptr(ptr: *mut u32) {
    WRITER.framebuffer.ptr = ptr;
}

pub fn clear() {
    unsafe {
        WRITER.clear();
    }
}

pub fn scroll() {
    unsafe {
        WRITER.scroll();
    }
}

// Macro magic (https://os.phil-opp.com/vga-text-mode/#a-println-macro)

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::writer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: Arguments) {
    unsafe { WRITER.write_fmt(args).unwrap(); }
}
