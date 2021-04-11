
use common::Framebuffer;

static FONT: &[[[u8; 8]; 16]; 128] = unsafe { core::mem::transmute(include_bytes!("../vgafont.bin")) };

static mut WRITER: Option<Writer> = None;

pub struct Writer {
    framebuffer: Framebuffer,
    cursor: (usize, usize)
}

pub fn init(framebuffer: Framebuffer) {
    unsafe { WRITER = Some(Writer { framebuffer, cursor: (0, 0) }); }
}

pub fn clear() {
    let mut writer = unsafe { &mut WRITER };
    let writer = writer.as_mut().unwrap();
    for y in 0..writer.framebuffer.resolution_y {
        for x in 0..writer.framebuffer.resolution_x {
            unsafe { (writer.framebuffer.ptr as *mut u32).add(y * writer.framebuffer.stride + x).write(0); }
        }
    }
    writer.cursor = (0, 0);
}

pub fn write_char(c: char) {
    let mut writer = unsafe { &mut WRITER };
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
                    unsafe { base.add((cy * 16 + y) * writer.framebuffer.stride + cx * 8 + x).write(color); }
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

pub fn write_str(string: &str) {
    for c in string.chars() {
        write_char(c);
    }
}

pub fn write_u64(mut val: u64) {
    if val == 0 {
        write_char('0');
    } else {
        let mut chars = ['\0'; 20];
        let mut i = 19;
        while val > 0 {
            let digit = val % 10;
            let c = char::from_digit(digit as _, 10).unwrap();
            chars[i] = c;
            val /= 10;
            i -= 1;
        }
        for i in i + 1..20 {
            write_char(chars[i]);
        }
    }
}

pub fn write_hex(val: u64) {
    for i in (0..16).rev() {
        let digit = (val >> i*4) & 0xF;
        let c = char::from_digit(digit as _, 16).unwrap();
        write_char(c)
    }
}
