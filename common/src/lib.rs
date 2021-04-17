#![no_std]

#[macro_use]
pub mod writer;

#[repr(C)]
pub struct MachineInfoC {
    framebuffer: Framebuffer,
    xhci_base: u64,
    allocated_frames_len: usize,
    allocated_frames_ptr: *mut u8,
}

pub struct MachineInfo {
    pub framebuffer: Framebuffer,
    pub xhci_base: u64,
    pub allocated_frames: &'static mut [u8],
}

impl From<MachineInfoC> for MachineInfo {
    fn from(machine_info: MachineInfoC) -> Self {
        Self {
            framebuffer: machine_info.framebuffer,
            xhci_base: machine_info.xhci_base,
            allocated_frames: unsafe {
                core::slice::from_raw_parts_mut(
                    machine_info.allocated_frames_ptr,
                    machine_info.allocated_frames_len,
                )
            },
        }
    }
}

impl From<MachineInfo> for MachineInfoC {
    fn from(machine_info: MachineInfo) -> Self {
        let ptr = machine_info.allocated_frames.as_mut_ptr();
        let len = machine_info.allocated_frames.len();
        Self {
            framebuffer: machine_info.framebuffer,
            xhci_base: machine_info.xhci_base,
            allocated_frames_len: len,
            allocated_frames_ptr: ptr,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Framebuffer {
    pub ptr: *mut u32,
    pub resolution_x: usize,
    pub resolution_y: usize,
    pub stride: usize,
}

impl Framebuffer {
    pub fn new(ptr: *mut u32, resolution: (usize, usize), stride: usize) -> Self {
        Self {
            ptr,
            resolution_x: resolution.0,
            resolution_y: resolution.1,
            stride,
        }
    }

    pub fn resolution(&self) -> (usize, usize) {
        (self.resolution_x, self.resolution_y)
    }
}
