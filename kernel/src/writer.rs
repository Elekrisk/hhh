
use common::Framebuffer;

use crate::spinlock::Spinlock;

static FONT: &[[[u8; 8]; 16]; 128] = unsafe { core::mem::transmute(include_bytes!("../vgafont.bin")) };

static mut WRITER: Spinlock<Option<Writer>> = Spinlock::new(None);

pub struct Writer {
    framebuffer: Framebuffer,
    cursor: (usize, usize)
}

pub unsafe fn init(framebuffer: Framebuffer) {
    *WRITER.lock() = Some(Writer { framebuffer, cursor: (0, 0) });
}

pub unsafe fn clear() {
    let mut writer = WRITER.lock();
    let writer = writer.as_mut().unwrap();
    for y in 0..writer.framebuffer.resolution_y {
        for x in 0..writer.framebuffer.resolution_x {
            (writer.framebuffer.ptr as *mut u32).add(y * writer.framebuffer.stride + x).write(0);
        }
    }
    writer.cursor = (0, 0);
}

pub unsafe fn write_char(c: char) {
    let mut writer = WRITER.lock();
    let writer = writer.as_mut().unwrap();
    let glyph = FONT[c as usize];
    let (cy, cx) = writer.cursor;
    match c {
        '\n' => {
            writer.cursor = (cy + 1, 0);
        },
        o if o < ' ' => {},
        _ => {
            for y in 0..16 {
                for x in 0..8 {
                    let base = writer.framebuffer.ptr as *mut u32;
                    let grayscale = glyph[y][x];
                    let color = ((grayscale as u32) << 16) | ((grayscale as u32) << 8) | grayscale as u32;
                    base.add((cy * 16 + y) * writer.framebuffer.stride + cx * 8 + x).write(color);
                }
            }
            writer.cursor = if (cx + 1) * 8 >= writer.framebuffer.resolution_x {
                (cy + 1, 0)
            } else {
                (cy, cx + 1)
            };
        }
    }
}

pub unsafe fn write_str(string: &str) {
    for c in string.chars() {
        write_char(c);
    }
}
