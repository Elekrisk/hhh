#![no_std]

#[macro_use]
pub mod writer;

#[repr(C)]
pub struct MachineInfo {
    pub framebuffer: Framebuffer,
    pub xhci_base: u64
}

#[repr(C)]
#[derive(Clone)]
pub struct Framebuffer {
    pub ptr: *mut u32,
    pub resolution_x: usize,
    pub resolution_y: usize,
    pub stride: usize
}

impl Framebuffer {
    pub fn new(ptr: *mut u32, resolution: (usize, usize), stride: usize) -> Self {
        Self {
            ptr,
            resolution_x: resolution.0,
            resolution_y: resolution.1,
            stride
        }
    }

    pub fn resolution(&self) -> (usize, usize) {
        (self.resolution_x, self.resolution_y)
    }
}
