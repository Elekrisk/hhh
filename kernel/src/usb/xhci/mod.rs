
const U32_SIZE: usize = core::mem::size_of::<u32>();

#[derive(Clone, Copy)]
struct Pointer(*mut u32);

impl Pointer {
    unsafe fn read_offset(self, byte_offset: usize) -> u32 {
        self.0.add(byte_offset / U32_SIZE).read_volatile()
    }
    unsafe fn write_offset(self, byte_offset: usize, value: u32) {
        self.0.add(byte_offset / U32_SIZE).write_volatile(value);
    }
    unsafe fn modify_offset<F: FnOnce(&mut u32)>(self, byte_offset: usize, modify_fun: F) {
        let ptr = self.0.add(byte_offset / U32_SIZE);
        let mut value = ptr.read_volatile();
        modify_fun(&mut value);
        ptr.write_volatile(value);
    }
    unsafe fn read_offset_64(self, byte_offset: usize) -> u64 {
        (self.0.add(byte_offset / U32_SIZE) as *mut u64).read_volatile()
    }
    unsafe fn write_offset_64(self, byte_offset: usize, value: u64) {
        (self.0.add(byte_offset / U32_SIZE) as *mut u64).write_volatile(value);
    }
    unsafe fn modify_offset_64<F: FnOnce(&mut u64)>(self, byte_offset: usize, modify_fun: F) {
        let ptr = self.0.add(byte_offset / U32_SIZE) as *mut u64;
        let mut value = ptr.read_volatile();
        modify_fun(&mut value);
        ptr.write_volatile(value);
    }
}

struct Pointers {
    capability: *mut u32,
    operational: *mut u32,
    runtime: *mut u32,
    doorbell: *mut u32
}

pub struct Operationals {
    base: Pointer
}

impl Operationals {
    unsafe fn write_usb_command<F: FnOnce(&mut u32)>(&mut self, fun: F) {
        self.base.modify_offset(0, fun);
    }

    // -- USBCMD --

    /// Halts the execution of the active schedule.
    ///
    /// Halting execution may cause events to get lost
    /// if any event rings are full.
    unsafe fn halt_execution(&mut self) {
        self.write_usb_command(|v| v.reset(0));
    }

    /// Starts the execution of the active schedule.
    ///
    /// The hardware must be halted.
    /// This can be checked with `.is_halted()`,
    /// and can be waited for with `.spin_until_halted()`.
    unsafe fn start_execution(&mut self) {
        self.write_usb_command(|v| v.set(0));
    }

    /// Reset the hardware. The software must then initialize
    /// it again. 
    ///
    /// The hardware must be halted.
    /// This can be checked with `.is_halted()`,
    /// and can be waited for with `.spin_until_halted()`.
    unsafe fn start_reset(&mut self) {
        self.write_usb_command(|v| v.set(1));

        // Spin until hardware reset is complete
        loop {
            let status = self.base.0.read_volatile();
            if status | 1 << 1 == 0 { break; }
        }
    }

    /// Enables interrupts from the controller.
    unsafe fn enable_interrupts(&mut self) {
        self.write_usb_command(|v| v.set(2));
    }

    /// Disables interrupts from the controller.
    unsafe fn disable_interrupts(&mut self) {
        self.write_usb_command(|v| v.reset(2));
    }

    unsafe fn enable_host_system_errors(&mut self) {
        self.write_usb_command(|v| v.set(3));
    }

    unsafe fn disable_host_system_errors(&mut self) {
        self.write_usb_command(|v| v.reset(3));
    }

    unsafe fn light_reset(&mut self) {
        self.write_usb_command(|v| v.set(7));

        // Spin until hardware reset is complete
        loop {
            let status = self.base.0.read_volatile();
            if !status.is_set(7) { break; }
        }
    }

    unsafe fn save_state(&mut self) {
        self.write_usb_command(|v| v.set(8));

        // TODO: spin for completion
    }

    unsafe fn restore_state(&mut self) {
        self.write_usb_command(|v| v.set(9));

        // TODO: spin for completion
    }

    unsafe fn enable_wrap_event(&mut self) {
        self.write_usb_command(|v| v.set(10));
    }

    unsafe fn disable_wrap_event(&mut self) {
        self.write_usb_command(|v| v.reset(10));
    }

    unsafe fn enable_u3_mfindex_stop(&mut self) {
        self.write_usb_command(|v| v.set(11));
    }

    unsafe fn disable_u3_mfindex_stop(&mut self) {
        self.write_usb_command(|v| v.reset(11));
    }

    unsafe fn enable_cem(&mut self) {
        self.write_usb_command(|v| v.set(13));
    }

    unsafe fn disable_cem(&mut self) {
        self.write_usb_command(|v| v.reset(13));
    }

    // TODO: add en/disable for bits 14-15?



    unsafe fn enable_vtio(&mut self) {
        self.write_usb_command(|v| v.set(16));
    }

    unsafe fn disable_vtio(&mut self) {
        self.write_usb_command(|v| v.reset(16));
    }


    // -- USBSTS --

    unsafe fn is_halted(&self) -> bool {
        self.base.read_offset(4).is_set(0)
    }

    unsafe fn system_error(&self) -> bool {
        self.base.read_offset(4).is_set(2)
    }

    unsafe fn event_interrupt(&self) -> bool {
        self.base.read_offset(4).is_set(3)
    }

    unsafe fn clear_event_interrupt(&mut self) {
        self.base.modify_offset(4, |v| v.set(3));
    }

    unsafe fn port_change_detected(&self) -> bool {
        self.base.read_offset(4).is_set(4)
    }

    unsafe fn clear_port_change_detected(&mut self) {
        self.base.modify_offset(4, |v| v.set(4));
    }

    unsafe fn currently_saving(&self) -> bool {
        self.base.read_offset(4).is_set(8)
    }

    unsafe fn currently_restoring(&self) -> bool {
        self.base.read_offset(4).is_set(9)
    }

    unsafe fn save_restore_error(&self) -> bool {
        self.base.read_offset(4).is_set(10)
    }

    unsafe fn controller_ready(&self) -> bool {
        self.base.read_offset(4).is_reset(11)
    }

    unsafe fn host_controller_error(&self) -> bool {
        self.base.read_offset(4).is_set(12)
    }

    
    // -- PAGESIZE --

    unsafe fn page_sizes_supported(&self) -> u16 {
        (self.base.read_offset(8) & 0xFF) as u16
    }


    // -- DNCTRL --

    unsafe fn enable_notification(&self, bit: u8) {
        self.base.modify_offset(8, |v| v.set(bit));
    }

    unsafe fn disable_notification(&self, bit: u8) {
        self.base.modify_offset(8, |v| v.reset(bit));
    }


    // -- CRCR

    unsafe fn 
}

pub struct XhciDriver {
    pointers: Pointers,
    max_device_slots: u8,
    max_interrupters: u16,
    max_ports: u8,
    iochronous_scheduling_threshold: u8,
    erst_max: u8,
    max_scratchpad_buffers: u16,
    scratchpad_restore: bool
}

impl XhciDriver {
    pub unsafe fn new(base: *mut u32) -> Self {
        let capabilities = Pointer(base);
        let cap_length = capabilities.read_offset(0) & 0xFF;
        let operational = Pointer(base.add(cap_length as usize / U32_SIZE));
        let runtime_offset = capabilities.read_offset(0x18);
        let runtime = Pointer(base.add(runtime_offset as usize / U32_SIZE));
        let doorbell_offset = capabilities.read_offset(0x14);
        let doorbell = Pointer(base.add(doorbell_offset as usize / U32_SIZE));

        let hcsparams1 = capabilities.read_offset(0x4);
        let max_device_slots = (hcsparams1 & 0xFF) as u8;
        let max_interrupters = (hcsparams1 >> 8 & 0x7FF) as u16;
        let max_ports = (hcsparams1 >> 24 & 0xFF) as u8; 

        let hcsparams2 = capabilities.read_offset(0x8);
        let isochronous_scheduling_threshold = (hcsparams2 & 0xF) as u8;
        let erst_max = (hcsparams2 >> 4 & 0xF) as u8;
        let scb_hi = (hcsparams2 >> 21 & 0x1F) as u8;
        let scratchpad_restore = (hcsparams2 >> 26 & 1) == 1;
        let scb_lo = (hcsparams2 >> 27) as u8;
        let max_scratchpad_buffers = scb_lo as u16 | (scb_hi as u16) << 5;

        let hcsparams3 = capabilities.read_offset(0xC);
        let u1_device_exit_latency = (hcsparams3 & 0xFF) as u8;
        let u2_device_exit_latency = (hcsparams3 >> 16 & 0xFFFF) as u16;

        let hccparams1 = capabilities.read_offset(0x10);
        let ac64 = hccparams1 & 1 > 0;
        let bandwith_negotiation = hccparams1 & (1<<1) > 0;
        let context_size = hccparams1 & (1<<2) > 0;
        let port_power_control = hccparams1 & (1<<3) > 0;
        let port_indicators = hccparams1 & (1<<4) > 0;
        let light_hc_reset = hccparams1 & (1<<5) > 0;
        let latency_tolerance_messaging = hccparams1 & (1<<6) > 0;
        let secondary_sid = hccparams1 & (1<<7) == 0; // inverted
        let parse_all_event_data = hccparams1 & (1<<8) > 0;
        let stopped_short_packet = hccparams1 & (1<<9) > 0;
        let stopped_edtla = hccparams1 & (1<<10) > 0;
        let contiguous_frame_id = hccparams1 & (1<<11) > 0;
        let max_primary_stream_array_size = (hccparams1 >> 12 & 0xF) as u8;
        let xhci_extented_capabilities_offset = (hccparams1 >> 16 & 0xFFFFFF) << 2;

        let hccparams2 = capabilities.read_offset(0x1C);
        let u3_entry = hccparams2 & 1 > 0;
        let configure_endpoint_command_max_exit_latency_too_large = hccparams2 & (1<<2) > 0;
        let force_save_context = hccparams2 & (1<<2) > 0;
        let compliance_transition = hccparams2 & (1<<3) > 0;
        let large_esit_payload = hccparams2 & (1<<4) > 0;
        let configuration_information_capability = hccparams2 & (1<<5) > 0;
        let extended_tbc = hccparams2 & (1<<6) > 0;
        let extended_tbc_trb_status = hccparams2 & (1<<7) > 0;
        let get_set_extended_property = hccparams2 & (1<<8) > 0;
        let virtualization_based_trusted_io = hccparams2 & (1<<9) > 0;

        let vtio_register_space_offset = hccparams2 >> 12;

        todo!()
    }
}


trait Flag {
    fn is_set(&self, bit: u8) -> bool;
    fn is_reset(&self, bit: u8) -> bool;
    fn set(&mut self, bit: u8);
    fn reset(&mut self, bit: u8);
    fn set_to(&mut self, bit: u8, value: bool);
}

impl Flag for u32 {
    fn is_set(&self, bit: u8) -> bool {
        *self | 1 << bit > 0
    }

    fn is_reset(&self, bit: u8) -> bool {
        *self | 1 << bit == 0
    }

    fn set(&mut self, bit: u8) {
        *self |= 1 << bit;
    }

    fn reset(&mut self, bit: u8) {
        *self &= !(1 << bit);
    }

    fn set_to(&mut self, bit: u8, value: bool) {
        if value == true {
            self.set(bit);
        } else {
            self.reset(bit);
        }
    }
}
