use alloc::prelude::v1::*;
use common::Framebuffer;
use spin::Mutex;

static GRAPHICS: Mutex<Graphics> = Mutex::new(Graphics {
    addr: 0,
    resolution: (0, 0),
    stride: 0,
    pixel_buffer: vec![],
});

pub struct Graphics {
    addr: usize,
    resolution: (usize, usize),
    stride: usize,
    pixel_buffer: Vec<Pixel>,
}

#[derive(Clone, Copy)]
pub struct Rect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Rect {
    pub const fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Rect {
            x,
            y,
            width,
            height,
        }
    }
}

pub struct Point {
    x: usize,
    y: usize,
}

impl Point {
    pub const fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

pub enum Line {
    HorizontalLine { x: usize, y: usize, length: usize },
    VerticalLine { x: usize, y: usize, length: usize },
}

impl Line {
    pub const fn new_horizontal(x: usize, y: usize, length: usize) -> Self {
        Line::HorizontalLine { x, y, length }
    }
    pub const fn new_vertical(x: usize, y: usize, length: usize) -> Self {
        Line::VerticalLine { x, y, length }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Pixel {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    reserved: u8,
}

impl Pixel {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            reserved: 0,
        }
    }
}

impl From<Pixel> for u32 {
    fn from(color: Pixel) -> Self {
        unsafe { core::mem::transmute(color) }
    }
}

impl Graphics {
    pub fn draw_rect(&mut self, rect: Rect, color: Pixel) {
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                self.pixel_buffer[y * self.stride + x] = color;
            }
        }
    }

    pub fn draw_line(&mut self, line: Line, width: usize, color: Pixel) {
        match line {
            Line::HorizontalLine { x, y, length } => {
                for y in y - width / 2..y + (width + 1) / 2 {
                    for x in x..x + length {
                        self.pixel_buffer[y * self.stride + x] = color;
                    }
                }
            }
            Line::VerticalLine { x, y, length } => {
                for y in y..y + length {
                    for x in x - width / 2..x + (width + 1) / 2 {
                        self.pixel_buffer[y * self.stride + x] = color;
                    }
                }
            }
        }
    }

    fn write_buffer(&mut self) {
        let dst = self.addr as *mut u32;
        let src = self.pixel_buffer.as_ptr() as *const u32;
        unsafe { core::ptr::copy_nonoverlapping(src, dst, self.stride * self.resolution.1) }
    }
}

pub unsafe fn init(framebuffer: Framebuffer) {
    let mut graphics = GRAPHICS.lock();
    graphics.addr = framebuffer.ptr as _;
    graphics.resolution = framebuffer.resolution();
    graphics.stride = framebuffer.stride;
    graphics.pixel_buffer = vec![Pixel::new(0, 0, 0); graphics.stride * graphics.resolution.1];
}

pub fn draw<F: FnOnce(&mut Graphics)>(func: F) {
    let mut graphics = GRAPHICS.lock();
    func(&mut graphics);
    graphics.write_buffer()
}
