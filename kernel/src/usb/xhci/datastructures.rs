use core::{
    cell::UnsafeCell,
    ops::{Index, IndexMut},
};

use super::register::{Num, Register};

use alloc::prelude::v1::*;

#[repr(C, packed)]
struct TrbTemplate {
    parameter: u64,
    status: u32,
    control: u32,
}

///  Lookup table for accessing DeviceContext structures.
#[repr(C, align(4096))]
pub struct Dcbaa32 {
    scratchpad_bufer_addr: *mut (),
    device_contexts: [Option<Box<DeviceContext32>>; 255],
}

impl Dcbaa32 {
    const NONE: Option<Box<DeviceContext32>> = None;
    pub const fn new() -> Self {
        Self {
            scratchpad_bufer_addr: 0 as _,
            device_contexts: [Self::NONE; 255],
        }
    }
}
#[repr(C, align(4096))]
pub struct Dcbaa64 {
    scratchpad_bufer_addr: *mut (),
    device_contexts: [Option<Box<DeviceContext64>>; 255],
}

impl Dcbaa64 {
    const NONE: Option<Box<DeviceContext64>> = None;
    pub const fn new() -> Self {
        Self {
            scratchpad_bufer_addr: 0 as _,
            device_contexts: [Self::NONE; 255],
        }
    }
}

pub trait DcbaaWrapper {}

impl DcbaaWrapper for Dcbaa32 {}
impl DcbaaWrapper for Dcbaa64 {}

/// Contains device configuration and state information.
#[repr(C, align(4096))]
struct DeviceContext32 {
    slot_context: SlotContext,
    endpoint_contexts: [EndpointContext; 31],
}

impl Index<usize> for DeviceContext32 {
    type Output = EndpointContext;

    fn index(&self, index: usize) -> &Self::Output {
        &self.endpoint_contexts[index - 1]
    }
}

impl IndexMut<usize> for DeviceContext32 {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.endpoint_contexts[index - 1]
    }
}

#[repr(C, align(4096))]
struct DeviceContext64 {
    slot_context: SlotContext,
    padding: u32,
    endpoint_contexts: [EndpointContext; 62],
}

impl Index<usize> for DeviceContext64 {
    type Output = EndpointContext;

    fn index(&self, index: usize) -> &Self::Output {
        &self.endpoint_contexts[(index - 1) * 2]
    }
}

impl IndexMut<usize> for DeviceContext64 {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.endpoint_contexts[(index - 1) * 2]
    }
}

#[repr(C)]
struct SlotContext {
    register0: Register<u32>,
    register1: Register<u32>,
    register2: Register<u32>,
    register3: Register<u32>,
    reserved: [u32; 4],
}

impl SlotContext {
    // register0

    pub fn route_string(&self) -> u32 {
        unsafe { self.register0.read() & 0x7FF }
    }

    pub fn multi_tt(&self) -> bool {
        unsafe { self.register0.get_bit(25) }
    }

    pub fn hub(&self) -> bool {
        unsafe { self.register0.get_bit(26) }
    }

    pub fn context_entries(&self) -> u8 {
        (unsafe { self.register0.read() } >> 27) as u8
    }

    // register1

    pub fn max_exit_latency(&self) -> u16 {
        (unsafe { self.register1.read() } & 0xFFFFF) as u16
    }

    pub fn root_hub_number(&self) -> u8 {
        (unsafe { self.register1.read() } >> 16 & 0xFF) as u8
    }

    pub fn number_of_ports(&self) -> u8 {
        (unsafe { self.register1.read() } >> 24) as u8
    }

    // register2

    pub fn parent_hub_slot_id(&self) -> u8 {
        (unsafe { self.register2.read() } & 0xFF) as u8
    }

    pub fn parent_port_number(&self) -> u8 {
        (unsafe { self.register2.read() } >> 8 & 0xFF) as u8
    }

    pub fn tt_think_time(&self) -> u8 {
        (unsafe { self.register2.read() } >> 16 & 0b11) as u8
    }

    pub fn interrupter_target(&self) -> u16 {
        (unsafe { self.register2.read() } >> 22) as u16
    }

    // register3

    pub fn usb_device_address(&self) -> u8 {
        (unsafe { self.register3.read() } & 0xFF) as u8
    }

    pub fn slot_state(&self) -> u8 {
        (unsafe { self.register3.read() } >> 27) as u8
    }
}

#[repr(C)]
struct EndpointContext {
    data: Register<u32>,
}
