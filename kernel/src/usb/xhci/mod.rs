#![allow(dead_code)]

use alloc::prelude::v1::*;

use datastructures::{Dcbaa32, Dcbaa64, DcbaaWrapper};
use x86_64::VirtAddr;

use register::{Capability, Operational, Port};

mod datastructures;
#[macro_use]
mod register;



pub struct XhciDriver {
    capability: &'static mut Capability,
    operational: &'static mut Operational,
    ports: &'static mut [Port]
}

impl XhciDriver {
    pub unsafe fn new(mut base: *mut u32) -> Self {
        panic!("USB support is put on hold for now");

        if (base as u64) < 0xFFFFFF80_00000000 {
            base = (base as u64 | 0xFFFFFF80_00000000) as _;
        }

        crate::memory::map_phys_offset(VirtAddr::new(base as u64));

        let capability = (base as *mut Capability).as_mut().unwrap();

        let cap_length = capability.cap_length();
        let port_count = capability.max_ports();

        let operational = ((base as u64 + cap_length as u64) as *mut Operational).as_mut().unwrap();
        let ports = core::slice::from_raw_parts_mut((base as u64 + cap_length as u64 + 0x400) as *mut Port, port_count as _);

        let dcbaap = operational.device_context_base_address_array_pointer();
        let dcbaap = dcbaap | 0xFFFFFF80_00000000;
        crate::memory::map_phys_offset(VirtAddr::new(dcbaap));

        if capability.uses_64_bit_contexts() {
            println!("64-bit contexts");
        } else {
            println!("32-bit contexts");
        };
        
        let device_slots_enabled = operational.max_device_slots_enabled();
        println!("Max {} device slots are enabled", device_slots_enabled);

        let device_slot_count = capability.max_device_slots();
        let interrupter_count = capability.max_interrupters();

        println!("Interface version: {:x}", capability.interface_version());
        println!("Device slot count: {}", device_slot_count);
        println!("Interrupter count: {}", interrupter_count);
        println!("Port count: {}", port_count);

        let mut driver = XhciDriver {
            capability,
            operational,
            ports
        };

        for port in driver.ports.iter() {
            if port.current_connect_status() {
                println!("Found connected USB device!");
            }
        }


        driver.initialize();
        driver
    }

    pub unsafe fn initialize(&mut self) {
        while !self.operational.controller_ready() {}
        println!("Controller ready");

        // Use max 1 device right now
        self.operational.set_max_device_slots_enabled(1);
        let max_scratch_buffers = self.capability.max_scratchpad_buffers();
        println!("max scratch buffers: {}", max_scratch_buffers);

        if max_scratch_buffers > 0 {
            panic!("scratch buffers not yet supported");
        }

        let dcbaa = if self.capability.uses_64_bit_contexts() {
            // Box should give us the correct alignment, as we have specified the alignment
            // of the DCBAA to 4K, which will also guarantee that the structure is contained in one
            // physical frame.
            let dcbaa = Box::<Dcbaa64>::new(
                Dcbaa64::new()
            );
            dcbaa as Box<dyn DcbaaWrapper>
        } else {
            let dcbaa = Box::<Dcbaa32>::new(
                Dcbaa32::new()
            );
            dcbaa as Box<dyn DcbaaWrapper>
        };
        

        loop {}
    }
}
