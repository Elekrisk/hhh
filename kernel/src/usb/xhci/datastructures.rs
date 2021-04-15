use super::register::{Num, Register};


#[repr(C, packed)]
struct TrbTemplate {
    parameter: u64,
    status: u32,
    control: u32
}


///  Lookup table for accessing DeviceContext structures.
#[repr(C, align(4096))]
struct DeviceContextBaseAddressArray<T: Num> {
    contents: [*mut DeviceContext<T>; 256]
}

impl<T: Num> DeviceContextBaseAddressArray<T> {
    pub const fn new() -> Self {
        DeviceContextBaseAddressArray {
            contents: [0 as _; 256]
        }
    }
}


#[repr(C, align(4096))]
struct DeviceContextBaseAddressArrayWrapper;

impl DeviceContextBaseAddressArrayWrapper {

}

/// Contains device configuration and state information.
#[repr(C, align(4096))]
struct DeviceContext<T: Num> {
    slot_context: SlotContext<T>,
    endpoint_contexts: [EndpointContext<T>; 31]
}

#[repr(C)]
struct SlotContext<T: Num> {
    data: Register<T>
}

#[repr(C)]
struct EndpointContext<T: Num> {
    data: Register<T>
}