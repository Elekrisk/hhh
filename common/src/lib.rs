#![no_std]

#[repr(C)]
pub struct Framebuffer {
    pub ptr: *mut u8,
    pub resolution_x: usize,
    pub resolution_y: usize,
    pub stride: usize
}
