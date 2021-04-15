
#[repr(C, packed)]
struct TrbTemplate {
    parameter: u64,
    status: u32,
    control: u32
}


///  Lookup table for accessing DeviceContext structures.
#[repr(C, align(4096))]
struct DeviceContextBaseAddressArray {
    contents: [*mut DeviceContextWrapper; 256]
}

impl DeviceContextBaseAddressArray {
    pub const fn new() -> Self {
        DeviceContextBaseAddressArray {
            contents: [0 as _; 256]
        }
    }
}


#[repr(C, align(4096))]
struct DeviceContextWrapper;

/// Contains device configuration and state information.
#[repr(C, align(4096))]
struct DeviceContext32 {
    slot_context: SlotContext32,
    endpoint_contexts: [EndpointContext32; 31]
}

#[repr(C)]
struct SlotContext32 {

}

#[repr(C)]
struct EndpointContext32 {

}