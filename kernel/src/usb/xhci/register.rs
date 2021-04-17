use core::{
    cell::UnsafeCell,
    ops::{BitAnd, Shr},
};

// use core::{cell::UnsafeCell, convert::{TryFrom, TryInto}, fmt::Debug, marker::PhantomData, mem::{Discriminant, discriminant}, ops::{BitAnd, BitOr, Not, Shl, Shr}};

// pub struct MaskInfo {
//     bit_size: usize,
//     bit_offset: usize
// }

// impl MaskInfo {
//     pub const fn new(bit_size: usize, bit_offset: usize) -> Self {
//         Self {
//             bit_size,
//             bit_offset
//         }
//     }
// }

// pub trait Num: Copy + Sized + Not<Output=Self> + Shl<usize, Output=Self> + Shr<usize, Output=Self> + BitAnd<Self, Output=Self> + BitOr<Self, Output=Self> {
//     const ZERO: Self;
//     const BIT_WIDTH: usize;
// }

// impl Num for u8 {
//     const ZERO: Self = 0;

//     const BIT_WIDTH: usize = 8;
// }

// impl Num for u16 {
//     const ZERO: Self = 0;

//     const BIT_WIDTH: usize = 16;
// }

// impl Num for u32 {
//     const ZERO: Self = 0;

//     const BIT_WIDTH: usize = 32;
// }

// impl Num for u64 {
//     const ZERO: Self = 0;

//     const BIT_WIDTH: usize = 64;
// }

// #[repr(C)]
// pub struct Register<RegDef: RegisterDef> {
//     data: UnsafeCell<RegDef::Data>,
//     _marker: PhantomData<RegDef>
// }

// impl<RegDef: RegisterDef> Register<RegDef> {
//     pub fn read<K: RegisterPart<RegDef>>(&self) -> K::Output {
//         let mask_info = K::MASK_INFO;
//         let low_mask = !RegDef::Data::ZERO >> (RegDef::Data::BIT_WIDTH - mask_info.bit_size) as usize;
//         let val = unsafe { self.data.get().read_volatile() };
//         let val = (val >> mask_info.bit_offset as usize) & low_mask;
//         K::from_value(val)
//     }

//     pub fn write<K: Writable<RegDef>>(&self, val_to_write: K::Output) {
//         let mask_info = K::MASK_INFO;
//         let low_mask = (!RegDef::Data::ZERO >> RegDef::Data::BIT_WIDTH) << RegDef::Data::BIT_WIDTH;
//         let low_mask = !low_mask;
//         let mask = low_mask << mask_info.bit_offset;
//         let mask = !mask;
//         let val_to_write = K::to_value(val_to_write) << mask_info.bit_offset;
//         let val = unsafe { self.data.get().read_volatile() };
//         let val = val & mask;
//         let val = val | val_to_write;
//         unsafe { self.data.get().write_volatile(val); }
//     }
// }

// pub trait RegisterPart<RegDef: RegisterDef> {
//     type Output;

//     const MASK_INFO: MaskInfo;

//     fn from_value(val: RegDef::Data) -> Self::Output;
// }

// pub trait Writable<RegDef: RegisterDef>: RegisterPart<RegDef> {
//     fn to_value(out: Self::Output) -> RegDef::Data;
// }

// pub trait RegisterDef {
//     type Data: Num;

//     unsafe fn write(ptr: *mut Self::Data, val: Self::Data, mask: Self::Data);
// }

// macro_rules! reg {
//     ($regbase:ident $({ $regdef:ident $($flag:ident $($z:literal)?)? , $t:ty : $( $regpart:ident $($flags:ident)? $(: $output:ty ,)? $(($bw:literal, $bo:literal))? $($b:literal)? $({ $from:expr $(, $to:expr )?})?);* $(;)? })* ) => {
//         paste::paste! {
//             #[repr(C)]
//             pub struct $regbase {
//                 $(
//                     pub [<$regdef:snake>]: Register<$regdef>,
//                 )*
//             }
//         }
//         impl $regbase {
//             $(
//                 $(
//                     out_func!($regdef $regpart $($flags)? $(: $output)?);
//                 )*
//             )*
//         }
//         $(
//             reg_def!($regdef $($flag $($z)?)?, $t : $($regpart $($flags)? $(: $output ,)? $(($bw, $bo))? $($b)? $({ $from $(, $to)? })?);* ;);
//         )*
//     };
// }

// macro_rules! out_func {
//     ($regdef:ident $regpart:ident : $output:ty) => {
//         paste::paste! {
//             pub fn [<read_ $regpart:snake>](&self) -> $output {
//                 self.[<$regdef:snake>].read::<$regpart>()
//             }
//         }
//     };
//     ($regdef:ident $regpart:ident W : $output:ty) => {
//         paste::paste! {
//             pub fn [<read_ $regpart:snake>](&self) -> $output {
//                 self.[<$regdef:snake>].read::<$regpart>()
//             }
//             pub fn [<write_ $regpart:snake>](&self, val: $output) {
//                 self.[<$regdef:snake>].write::<$regpart>(val);
//             }
//         }
//     };
//     ($regdef:ident $regpart:ident F) => {
//         paste::paste! {
//             pub fn [<read_ $regpart:snake>](&self) -> bool {
//                 self.[<$regdef:snake>].read::<$regpart>()
//             }
//         }
//     };
//     ($regdef:ident $regpart:ident WF : $output:ty) => {
//         paste::paste! {
//             pub fn [<read_ $regpart:snake>](&self) -> bool {
//                 self.[<$regdef:snake>].read::<$regpart>()
//             }
//             pub fn [<write_ $regpart:snake>](&self, val: bool) {
//                 self.[<$regdef:snake>].write::<$regpart>(val);
//             }
//         }
//     };
// }

// macro_rules! reg_def {
//     ($regdef:ident, $t:ty : $( $regpart:ident $($flags:ident)? $(: $output:ty ,)? $(($bw:literal, $bo:literal))? $($b:literal)? $({ $from:expr $(, $to:expr )?})?);* $(;)?) => {
//         pub struct $regdef;
//         impl crate::usb::xhci::register::RegisterDef for $regdef {
//             type Data = $t;

//             unsafe fn write(ptr: *mut Self::Data, val: Self::Data, mask: Self::Data) {
//                 let value = ptr.read_volatile();
//                 let value = value & mask;
//                 let value = value | val;
//                 ptr.write_volatile(value);
//             }
//         }
//         $(
//             reg_part2!($regdef - $regpart $($flags)? $(: $output ,)? $(($bw, $bo))? $($b)? $({ $from $(, $to)? })?);
//         )*
//     };
//     ($regdef:ident Z, $t:ty : $( $regpart:ident $($flags:ident)? $(: $output:ty ,)? $(($bw:literal, $bo:literal))? $($b:literal)? $({ $from:expr $(, $to:expr )?})?);* $(;)?) => {
//         pub struct $regdef;
//         impl crate::usb::xhci::register::RegisterDef for $regdef {
//             type Data = $t;

//             unsafe fn write(ptr: *mut Self::Data, val: Self::Data, _mask: Self::Data) {
//                 ptr.write_volatile(val);
//             }
//         }
//         $(
//             reg_part2!($regdef - $regpart $($flags)? $(: $output ,)? $(($bw, $bo))? $($b)? $({ $from $(, $to)? })?);
//         )*
//     };
//     ($regdef:ident M $z:literal, $t:ty : $( $regpart:ident $($flags:ident)? $(: $output:ty ,)? $(($bw:literal, $bo:literal))? $($b:literal)? $({ $from:expr $(, $to:expr )?})?);* $(;)?) => {
//         pub struct $regdef;
//         impl crate::usb::xhci::register::RegisterDef for $regdef {
//             type Data = $t;

//             unsafe fn write(ptr: *mut Self::Data, val: Self::Data, mask: Self::Data) {
//                 let value = ptr.read_volatile();
//                 let value = value & mask;
//                 let value = value | !$z;
//                 let value = value | val;
//                 ptr.write_volatile(value);
//             }
//         }
//         $(
//             reg_part2!($regdef - $regpart $($flags)? $(: $output ,)? $(($bw, $bo))? $($b)? $({ $from $(, $to)? })?);
//         )*
//     };
// }

// macro_rules! reg_part2 {
//     ($regdef:ident - $regpart:ident : $output:ty, ($bw:literal, $bo:literal)) => {
//         pub struct $regpart;
//         impl crate::usb::xhci::register::RegisterPart<$regdef> for $regpart {
//             type Output = $output;

//             const MASK_INFO: crate::usb::xhci::register::MaskInfo = crate::usb::xhci::register::MaskInfo::new($bw, $bo);

//             fn from_value(val: <$regdef as crate::usb::xhci::register::RegisterDef>::Data) -> Self::Output {
//                 val as _
//             }
//         }
//     };
//     ($regdef:ident - $regpart:ident : $output:ty, ($bw:literal, $bo:literal) { $to:expr }) => {
//         impl $regdef {
//             pub type $regpart = $regpart;
//         }
//         pub struct $regpart;
//         impl crate::usb::xhci::register::RegisterPart<$regdef> for $regpart {
//             type Output = $output;

//             const MASK_INFO: crate::usb::xhci::register::MaskInfo = crate::usb::xhci::register::MaskInfo::new($bw, $bo);

//             fn from_value(val: <$regdef as crate::usb::xhci::register::RegisterDef>::Data) -> Self::Output {
//                 ($to)(val)
//             }
//         }
//     };
//     ($regdef:ident - $regpart:ident W : $output:ty, ($bw:literal, $bo:literal)) => {
//         impl $regdef {
//             pub type $regpart = $regpart;
//         }
//         pub struct $regpart;
//         impl crate::usb::xhci::register::RegisterPart<$regdef> for $regpart {
//             type Output = $output;

//             const MASK_INFO: crate::usb::xhci::register::MaskInfo = crate::usb::xhci::register::MaskInfo::new($bw, $bo);

//             fn from_value(val: <$regdef as crate::usb::xhci::register::RegisterDef>::Data) -> Self::Output {
//                 val as _
//             }
//         }
//         impl crate::usb::xhci::register::Writable<$regdef> for $regpart {
//             fn to_value(out: Self::Output) -> <$regdef as crate::usb::xhci::register::RegisterDef>::Data {
//                 out as _
//             }
//         }
//     };
//     ($regdef:ident - $regpart:ident W : $output:ty, ($bw:literal, $bo:literal) { $to:expr, $from:expr }) => {
//         impl $regdef {
//             pub type $regpart = $regpart;
//         }
//         pub struct $regpart;
//         impl crate::usb::xhci::register::RegisterPart<$regdef> for $regpart {
//             type Output = $output;

//             const MASK_INFO: crate::usb::xhci::register::MaskInfo = crate::usb::xhci::register::MaskInfo::new($bw, $bo);

//             fn from_value(val: <$regdef as crate::usb::xhci::register::RegisterDef>::Data) -> Self::Output {
//                 ($to)(val)
//             }
//         }
//         impl crate::usb::xhci::register::Writable<$regdef> for $regpart {
//             fn to_value(out: Self::Output) -> <$regdef as crate::usb::xhci::register::RegisterDef>::Data {
//                 ($from)(out)
//             }
//         }
//     };
//     ($regdef:ident - $regpart:ident F $b:literal) => {
//         impl $regdef {
//             pub type $regpart = $regpart;
//         }
//         pub struct $regpart;
//         impl crate::usb::xhci::register::RegisterPart<$regdef> for $regpart {
//             type Output = bool;

//             const MASK_INFO: crate::usb::xhci::register::MaskInfo = crate::usb::xhci::register::MaskInfo::new(1, $b);

//             fn from_value(val: <$regdef as crate::usb::xhci::register::RegisterDef>::Data) -> Self::Output {
//                 val > 0
//             }
//         }
//     };
//     ($regdef:ident - $regpart:ident WF $b:literal) => {
//         impl $regdef {
//             pub type $regpart = $regpart;
//         }
//         pub struct $regpart;
//         impl crate::usb::xhci::register::RegisterPart<$regdef> for $regpart {
//             type Output = bool;

//             const MASK_INFO: crate::usb::xhci::register::MaskInfo = crate::usb::xhci::register::MaskInfo::new(1, $b);

//             fn from_value(val: <$regdef as crate::usb::xhci::register::RegisterDef>::Data) -> Self::Output {
//                 val > 0
//             }
//         }
//         impl crate::usb::xhci::register::Writable<$regdef> for $regpart {
//             fn to_value(out: Self::Output) -> <$regdef as crate::usb::xhci::register::RegisterDef>::Data {
//                 out as _
//             }
//         }
//     };
// }

#[repr(transparent)]
pub struct Register<T: Copy = u32> {
    data: UnsafeCell<T>,
}

impl<T: Copy + Num> Register<T> {
    pub unsafe fn read(&self) -> T {
        self.data.get().read_volatile()
    }

    pub unsafe fn write(&mut self, value: T) {
        self.data.get().write_volatile(value);
    }
}

impl<T: Copy + Num> Register<T> {
    pub unsafe fn get_bit(&self, bit: usize) -> bool {
        self.data.get().read_volatile() >> bit & T::ONE == T::ONE
    }
}

pub trait Num: Eq + Copy + Sized + Shr<usize, Output = Self> + BitAnd<Self, Output = Self> {
    const ONE: Self;
}

impl Num for u32 {
    const ONE: Self = 1;
}

impl Num for u64 {
    const ONE: Self = 1;
}

#[repr(C)]
pub struct Capability {
    first: Register,
    hcsparams1: Register,
    hcsparams2: Register,
    hcsparams3: Register,
    hccparams1: Register,
    dboff: Register,
    rtsoff: Register,
    hccparams2: Register,
    vtiosoff: Register,
}

impl Capability {
    pub fn cap_length(&self) -> u8 {
        (unsafe { self.first.read() } & 0xFF) as u8
    }

    pub fn interface_version(&self) -> u16 {
        (unsafe { self.first.read() } >> 16 & 0xFFFFF) as u16
    }

    // hcsparams1

    pub fn max_device_slots(&self) -> u8 {
        (unsafe { self.hcsparams1.read() } & 0xFF) as u8
    }

    pub fn max_interrupters(&self) -> u16 {
        (unsafe { self.hcsparams1.read() } >> 8 & 0x7FF) as u16
    }

    pub fn max_ports(&self) -> u8 {
        (unsafe { self.hcsparams1.read() } >> 24) as u8
    }

    // hcsparams2

    pub fn ist(&self) -> u8 {
        (unsafe { self.hcsparams2.read() } & 0xF) as u8
    }

    pub fn erst_max(&self) -> u8 {
        (unsafe { self.hcsparams2.read() } >> 4 & 0xF) as u8
    }

    pub fn scratchpad_restore(&self) -> bool {
        (unsafe { self.hcsparams2.read() } >> 26 & 1) == 1
    }

    pub fn max_scratchpad_buffers(&self) -> u16 {
        let low = (unsafe { self.hcsparams2.read() } >> 21 & 0x1F) as u16;
        let high = (unsafe { self.hcsparams2.read() } >> 27 & 0x1F) as u16;
        low | high << 5
    }

    // hcsparams3

    pub fn u1_device_exit_latency(&self) -> u8 {
        (unsafe { self.hcsparams3.read() } & 0xFF) as u8
    }

    pub fn u2_device_exit_latency(&self) -> u16 {
        (unsafe { self.hcsparams3.read() } >> 16) as u16
    }

    // hccparams1

    pub fn uses_64_bit_addresses(&self) -> bool {
        (unsafe { self.hccparams1.read() } & 1) == 1
    }

    pub fn bandwidth_negotiation(&self) -> bool {
        (unsafe { self.hccparams1.read() } >> 1 & 1) == 1
    }

    pub fn uses_64_bit_contexts(&self) -> bool {
        (unsafe { self.hccparams1.read() } >> 2 & 1) == 1
    }

    pub fn port_power_control(&self) -> bool {
        (unsafe { self.hccparams1.read() } >> 3 & 1) == 1
    }

    pub fn port_indicators_control(&self) -> bool {
        (unsafe { self.hccparams1.read() } >> 4 & 1) == 1
    }

    pub fn light_hc_reset(&self) -> bool {
        (unsafe { self.hccparams1.read() } >> 5 & 1) == 1
    }

    pub fn latency_tolerance_messaging(&self) -> bool {
        (unsafe { self.hccparams1.read() } >> 6 & 1) == 1
    }

    pub fn secondary_sid(&self) -> bool {
        (unsafe { self.hccparams1.read() } >> 7 & 1) == 0
    }

    pub fn parse_all_event_data(&self) -> bool {
        (unsafe { self.hccparams1.read() } >> 8 & 1) == 1
    }

    pub fn stopped_short_packet(&self) -> bool {
        (unsafe { self.hccparams1.read() } >> 9 & 1) == 1
    }

    pub fn stopped_edtla(&self) -> bool {
        (unsafe { self.hccparams1.read() } >> 10 & 1) == 1
    }

    pub fn contiguous_frame_id(&self) -> bool {
        (unsafe { self.hccparams1.read() } >> 11 & 1) == 1
    }

    pub fn max_primary_stream_array_size(&self) -> u8 {
        (unsafe { self.hccparams1.read() } >> 12 & 0xF) as u8
    }

    pub fn xhci_extended_capabilities_pointer(&self) -> u32 {
        ((unsafe { self.hccparams1.read() } >> 16 & 0xFFFF) as u32) << 2
    }

    // dboff

    pub fn doorbell_offset(&self) -> u32 {
        unsafe { self.dboff.read() }
    }

    // rtsoff

    pub fn runtime_register_space_offset(&self) -> u32 {
        unsafe { self.rtsoff.read() }
    }

    // hccparams2

    pub fn u3_entry(&self) -> bool {
        (unsafe { self.hccparams2.read() }) & 1 == 1
    }

    pub fn configure_endpoint_command_max_exit_latency_too_large(&self) -> bool {
        (unsafe { self.hccparams2.read() } >> 1 & 1) == 1
    }

    pub fn force_save_context(&self) -> bool {
        (unsafe { self.hccparams2.read() } >> 2 & 1) == 1
    }

    pub fn compliance_transition(&self) -> bool {
        (unsafe { self.hccparams2.read() } >> 3 & 1) == 1
    }

    pub fn large_esit_payload(&self) -> bool {
        (unsafe { self.hccparams2.read() } >> 4 & 1) == 1
    }

    pub fn configuration_information(&self) -> bool {
        (unsafe { self.hccparams2.read() } >> 5 & 1) == 1
    }

    pub fn extended_tbc(&self) -> bool {
        (unsafe { self.hccparams2.read() } >> 6 & 1) == 1
    }

    pub fn extended_tbc_trb_status(&self) -> bool {
        (unsafe { self.hccparams2.read() } >> 7 & 1) == 1
    }

    pub fn get_set_extended_property(&self) -> bool {
        (unsafe { self.hccparams2.read() } >> 8 & 1) == 1
    }

    pub fn virtualization_based_trusted_io(&self) -> bool {
        (unsafe { self.hccparams2.read() } >> 9 & 1) == 1
    }

    // vtiosoff

    pub fn vtio_register_space_offset(&self) -> u32 {
        unsafe { self.vtiosoff.read() }
    }
}

#[repr(C)]
pub struct Operational {
    /// Write strategy: Preserve for all
    usbcmd: Register,
    /// Write strategy: Zero for all
    usbsts: Register,
    pagesize: Register,
    _reserved: [Register; 2],
    /// Write strategy: Preserve for all
    dnctrl: Register,
    /// Write strategy: Mixed
    crcr: Register<u64>,
    _reserved2: [Register; 4],
    dcbaap: Register<u64>,
    /// Write strategy: Preserve for all
    config: Register,
}

impl Operational {
    pub fn get_run_stop(&self) -> bool {
        (unsafe { self.usbcmd.read() } & 1) == 1
    }

    /// The software MUST check that the controller is halted
    /// before calling this function.
    pub unsafe fn start_running(&mut self) {
        let mut val = self.usbcmd.read();
        val |= 1;
        self.usbcmd.write(val);
    }

    /// Poll `.is_halted()` to wait for the controller to finish
    /// halting.
    ///
    /// If any event rings are full before calling this function,
    /// events may get lost.
    pub unsafe fn stop_running(&mut self) {
        let mut val = self.usbcmd.read();
        val &= !1;
        self.usbcmd.write(val);
    }

    pub fn interrupts_enabled(&self) -> bool {
        (unsafe { self.usbcmd.read() } >> 2 & 1) == 1
    }

    pub unsafe fn set_interrupts_enabled(&mut self, status: bool) {
        let mut val = self.usbcmd.read();
        if status {
            val |= 1 << 2;
        } else {
            val &= !(1 << 2);
        }
        self.usbcmd.write(val);
    }

    pub fn host_system_error_enabled(&self) -> bool {
        unsafe { self.usbcmd.get_bit(3) }
    }

    pub unsafe fn set_host_system_error_enabled(&mut self, status: bool) {
        let mut val = self.usbcmd.read();
        if status {
            val |= 1 << 3;
        } else {
            val &= !(1 << 3);
        }
        self.usbcmd.write(val);
    }

    pub fn light_host_controller_reset_complete(&self) -> bool {
        unsafe { !self.usbcmd.get_bit(7) }
    }

    pub unsafe fn light_host_controller_reset(&mut self) {
        let mut val = self.usbcmd.read();
        val |= 1 << 7;
        self.usbcmd.write(val);
    }

    pub unsafe fn controller_save_state(&mut self) {
        let mut val = self.usbcmd.read();
        val |= 1 << 8;
        self.usbcmd.write(val);
    }

    pub unsafe fn controller_restore_state(&mut self) {
        let mut val = self.usbcmd.read();
        val |= 1 << 9;
        self.usbcmd.write(val);
    }

    pub fn wrap_event_enabled(&self) -> bool {
        unsafe { self.usbcmd.get_bit(10) }
    }

    pub unsafe fn set_wrap_event_enabled(&mut self, status: bool) {
        let mut val = self.usbcmd.read();
        if status {
            val |= 1 << 10;
        } else {
            val &= !(1 << 10);
        }
        self.usbcmd.write(val);
    }

    pub fn u3_mfindex_stop_enabled(&self) -> bool {
        unsafe { self.usbcmd.get_bit(11) }
    }

    pub unsafe fn set_u32_mfindex_stop_enabled(&mut self, status: bool) {
        let mut val = self.usbcmd.read();
        if status {
            val |= 1 << 11;
        } else {
            val &= !(1 << 11);
        }
        self.usbcmd.write(val);
    }

    pub fn cem_enabled(&self) -> bool {
        unsafe { self.usbcmd.get_bit(13) }
    }

    pub unsafe fn set_cem_enabled(&mut self, status: bool) {
        let mut val = self.usbcmd.read();
        if status {
            val |= 1 << 13;
        } else {
            val &= !(1 << 13);
        }
        self.usbcmd.write(val);
    }

    pub fn extended_tbc_enabled(&self) -> bool {
        unsafe { self.usbcmd.get_bit(14) }
    }

    pub unsafe fn set_extended_tbc_enabled(&mut self, status: bool) {
        let mut val = self.usbcmd.read();
        if status {
            val |= 1 << 14;
        } else {
            val &= !(1 << 14);
        }
        self.usbcmd.write(val);
    }

    pub fn extended_tbc_trb_status_enabled(&self) -> bool {
        unsafe { self.usbcmd.get_bit(15) }
    }

    pub unsafe fn set_extended_tbc_trb_status_enabled(&mut self, status: bool) {
        let mut val = self.usbcmd.read();
        if status {
            val |= 1 << 15;
        } else {
            val &= !(1 << 15);
        }
        self.usbcmd.write(val);
    }

    pub fn vtio_enabled(&self) -> bool {
        unsafe { self.usbcmd.get_bit(16) }
    }

    pub unsafe fn set_vtio_enabled(&mut self, status: bool) {
        let mut val = self.usbcmd.read();
        if status {
            val |= 1 << 16;
        } else {
            val &= !(1 << 16);
        }
        self.usbcmd.write(val);
    }

    // USBSTS

    pub fn is_halted(&self) -> bool {
        unsafe { self.usbsts.get_bit(0) }
    }

    pub fn host_system_error(&self) -> bool {
        unsafe { self.usbsts.get_bit(2) }
    }

    pub unsafe fn clear_host_system_error(&mut self) {
        self.usbsts.write(1 << 2);
    }

    pub fn event_interrupt(&self) -> bool {
        unsafe { self.usbsts.get_bit(3) }
    }

    pub unsafe fn clear_event_interrupt(&mut self) {
        self.usbsts.write(1 << 3);
    }

    pub fn port_change_detect(&self) -> bool {
        unsafe { self.usbsts.get_bit(4) }
    }

    pub unsafe fn clear_port_change_detect(&mut self) {
        self.usbsts.write(1 << 4);
    }

    pub fn save_state_status(&self) -> bool {
        unsafe { self.usbsts.get_bit(8) }
    }

    pub fn restore_state_status(&self) -> bool {
        unsafe { self.usbsts.get_bit(9) }
    }

    pub fn save_restore_error(&self) -> bool {
        unsafe { self.usbsts.get_bit(10) }
    }

    pub unsafe fn clear_save_restore_error(&mut self) {
        self.usbsts.write(1 << 10);
    }

    pub fn controller_ready(&self) -> bool {
        unsafe { !self.usbsts.get_bit(11) }
    }

    pub fn host_controller_error(&self) -> bool {
        unsafe { !self.usbsts.get_bit(12) }
    }

    // pagesize

    pub fn page_size(&self) -> u32 {
        (unsafe { self.pagesize.read() } & 0xFFFFFF) << 12
    }

    // dnctrl

    pub fn notification_enabled(&mut self, notification_type: usize) -> bool {
        assert!(notification_type <= 15);
        unsafe { self.dnctrl.get_bit(notification_type) }
    }

    pub unsafe fn set_notification_enabled(&mut self, notification_type: usize, status: bool) {
        let mut val = self.dnctrl.read();
        if status {
            val |= 1 << notification_type;
        } else {
            val &= !(1 << notification_type);
        }
        self.dnctrl.write(val);
    }

    // crcr

    pub unsafe fn set_ring_cycle_state(&mut self, rcs: bool, crp: u64) {
        let mut val = self.crcr.read();
        // bit 0 (RCS) is RW
        // bits 1:2 (CS, CA) are RW1S and are thus set to 0
        // bit 3 is RO
        // Preserve 4:5
        // bits 6:31 (CRP) is RW
        val &= 0b11 << 4;
        val |= rcs as u64;
        val |= crp;
        self.crcr.write(val);
    }

    pub unsafe fn stop_command(&mut self) {
        let mut val = self.crcr.read();
        val &= 0b11 << 4;
        val |= 1 << 1;
        self.crcr.write(val);
    }

    pub unsafe fn abort_command(&mut self) {
        let mut val = self.crcr.read();
        val &= 0b11 << 4;
        val |= 1 << 2;
        self.crcr.write(val);
    }

    // dcbaap

    pub fn device_context_base_address_array_pointer(&self) -> u64 {
        unsafe { self.dcbaap.read() }
    }

    pub unsafe fn set_device_context_base_address_array_pointer(&mut self, ptr: u64) {
        self.dcbaap.write(ptr)
    }

    // config

    pub fn max_device_slots_enabled(&self) -> u8 {
        (unsafe { self.dcbaap.read() } & 0xFF) as u8
    }

    pub unsafe fn set_max_device_slots_enabled(&mut self, value: u8) {
        let mut val = self.config.read();
        val &= !0xFF;
        val |= value as u32;
        self.config.write(val);
    }

    pub fn u32_entry_enabled(&self) -> bool {
        unsafe { self.config.get_bit(8) }
    }

    pub unsafe fn set_u32_entry_enabled(&mut self, status: bool) {
        let mut val = self.config.read();
        if status {
            val |= 1 << 8;
        } else {
            val &= !(1 << 8);
        }
        self.config.write(val);
    }

    pub fn configuration_information_enabled(&self) -> bool {
        unsafe { self.config.get_bit(8) }
    }

    pub unsafe fn set_configuration_information_enabled(&mut self, status: bool) {
        let mut val = self.config.read();
        if status {
            val |= 1 << 8;
        } else {
            val &= !(1 << 8);
        }
        self.config.write(val);
    }
}

#[repr(C)]
pub struct Port {
    /// Write strategy: Mixed
    ///
    /// ```ignore
    /// preserve-mask 0xE00C200
    /// 0   z
    /// 1   z
    /// 2   z
    /// 3   z
    /// 4   z
    /// 5   z
    /// 6   z
    /// 7   z
    /// 8   z
    /// 9   p
    /// 10  z
    /// 11  z
    /// 12  z
    /// 13  z
    /// 14  p
    /// 15  p
    /// 16  z
    /// 17  z
    /// 18  z
    /// 19  z
    /// 20  z
    /// 21  z
    /// 22  z
    /// 23  z
    /// 24  z
    /// 25  p
    /// 26  p
    /// 27  p
    /// 28  z
    /// 29  z
    /// 30  z
    /// 31  z
    /// ```
    portsc: Register,
    /// Write strategy: Preserve for all
    portpmsc: Register,
    portli: Register,
    porthlpmc: Register,
}

impl Port {
    // portsc
    const PORTSC_PMASK: u32 = 0x0E00C200;

    pub fn current_connect_status(&self) -> bool {
        unsafe { self.portsc.get_bit(0) }
    }

    pub fn port_enabled(&self) -> bool {
        unsafe { self.portsc.get_bit(1) }
    }

    // Bit 4 (PR) must be written with a value of 0
    pub unsafe fn disable_port(&mut self) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        val |= 1 << 1;
        self.portsc.write(val);
    }

    pub fn over_current_active(&self) -> bool {
        unsafe { self.portsc.get_bit(3) }
    }

    pub fn port_reset_status(&self) -> bool {
        unsafe { self.portsc.get_bit(4) }
    }

    pub unsafe fn reset_port(&mut self) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        val |= 1 << 4;
        self.portsc.write(val);
    }

    pub fn port_link_state(&self) -> u8 {
        (unsafe { self.portsc.read() } >> 5 & 0xF) as u8
    }

    pub unsafe fn set_port_link_state(&mut self, state: u8) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        val &= !(0xF << 5);
        val |= (state as u32 & 0xF) << 5;
        // Also set LWS so that the write is not ignored
        val |= 1 << 16;
        self.portsc.write(val);
    }

    pub fn port_power(&self) -> bool {
        unsafe { self.portsc.get_bit(9) }
    }

    pub unsafe fn set_port_power(&mut self, status: bool) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        if status {
            val |= 1 << 9;
        } else {
            val &= !(1 << 9);
        }
        self.portsc.write(val);
    }

    pub fn port_speed(&self) -> u8 {
        (unsafe { self.portsc.read() } >> 10 & 0xF) as u8
    }

    pub fn port_indicator(&self) -> u8 {
        (unsafe { self.portsc.read() } >> 14 & 0b11) as u8
    }

    pub unsafe fn set_port_indicator(&mut self, status: u8) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        val &= !(0b11 << 14);
        val |= (status as u32 & 0b11) << 14;
        self.portsc.write(val);
    }

    pub fn connect_status_change(&self) -> bool {
        unsafe { self.portsc.get_bit(17) }
    }

    pub unsafe fn clear_connect_status_change(&mut self) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        val |= 1 << 17;
        self.portsc.write(val);
    }

    pub fn port_enable_disable_change(&self) -> bool {
        unsafe { self.portsc.get_bit(18) }
    }

    pub unsafe fn clear_port_enable_disable_change(&mut self) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        val |= 1 << 18;
        self.portsc.write(val);
    }

    pub fn warm_port_reset_change(&self) -> bool {
        unsafe { self.portsc.get_bit(19) }
    }

    pub unsafe fn clear_warm_port_reset_change(&mut self) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        val |= 1 << 19;
        self.portsc.write(val);
    }

    pub fn over_current_change(&self) -> bool {
        unsafe { self.portsc.get_bit(20) }
    }

    pub unsafe fn clear_over_current_change(&mut self) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        val |= 1 << 20;
        self.portsc.write(val);
    }

    pub fn port_reset_change(&self) -> bool {
        unsafe { self.portsc.get_bit(21) }
    }

    pub unsafe fn clear_port_reset_change(&mut self) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        val |= 1 << 21;
        self.portsc.write(val);
    }

    pub fn port_link_state_change(&self) -> bool {
        unsafe { self.portsc.get_bit(22) }
    }

    pub unsafe fn clear_port_link_state_change(&mut self) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        val |= 1 << 22;
        self.portsc.write(val);
    }

    pub fn port_config_error_change(&self) -> bool {
        unsafe { self.portsc.get_bit(23) }
    }

    pub unsafe fn clear_port_config_error_change(&mut self) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        val |= 1 << 23;
        self.portsc.write(val);
    }

    pub fn cold_attach_status(&self) -> bool {
        unsafe { self.portsc.get_bit(24) }
    }

    pub fn wake_on_connect_enabled(&self) -> bool {
        unsafe { self.portsc.get_bit(25) }
    }

    pub unsafe fn set_wake_on_connect_enabled(&mut self, status: bool) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        if status {
            val |= 1 << 25;
        } else {
            val &= !(1 << 25);
        }
        self.portsc.write(val);
    }

    pub fn wake_on_disconnect_enabled(&self) -> bool {
        unsafe { self.portsc.get_bit(26) }
    }

    pub unsafe fn set_wake_on_disconnect_enabled(&mut self, status: bool) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        if status {
            val |= 1 << 26;
        } else {
            val &= !(1 << 26);
        }
        self.portsc.write(val);
    }

    pub fn wake_on_over_current_enabled(&self) -> bool {
        unsafe { self.portsc.get_bit(27) }
    }

    pub unsafe fn set_wake_on_over_current_enabled(&mut self, status: bool) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        if status {
            val |= 1 << 27;
        } else {
            val &= !(1 << 27);
        }
        self.portsc.write(val);
    }

    pub fn device_removable(&self) -> bool {
        unsafe { self.portsc.get_bit(30) }
    }

    pub unsafe fn warm_reset_port(&mut self) {
        let mut val = self.portsc.read();
        val &= Self::PORTSC_PMASK;
        val |= 1 << 31;
        self.portsc.write(val);
    }

    // portpmsc - usb3

    pub fn u1_timeout(&self) -> u8 {
        (unsafe { self.portpmsc.read() } & 0xFF) as u8
    }

    pub unsafe fn set_u1_timeout(&mut self, status: u8) {
        let mut val = self.portpmsc.read();
        val &= !0xFF;
        val |= status as u32;
        self.portpmsc.write(val);
    }

    pub fn u2_timeout(&self) -> u16 {
        (unsafe { self.portpmsc.read() } >> 8 & 0xFFFF) as u16
    }

    pub unsafe fn set_u2_timeout(&mut self, status: u16) {
        let mut val = self.portpmsc.read();
        val &= !(0xFFFF << 8);
        val |= (status as u32) << 8;
        self.portpmsc.write(val);
    }

    pub fn force_link_pm_accept(&self) -> bool {
        unsafe { self.portpmsc.get_bit(16) }
    }

    pub unsafe fn set_force_link_pm_accept(&mut self, status: bool) {
        let mut val = self.portpmsc.read();
        if status {
            val |= 1 << 16;
        } else {
            val &= !(1 << 16);
        }
        self.portpmsc.write(val);
    }

    // portpmsc - usb2

    pub fn l1_status(&self) -> u8 {
        (unsafe { self.portpmsc.read() } & 0b111) as u8
    }

    pub fn remote_wake_enabled(&self) -> bool {
        unsafe { self.portpmsc.get_bit(3) }
    }

    pub unsafe fn set_remote_wake_enabled(&mut self, status: bool) {
        let mut val = self.portpmsc.read();
        if status {
            val |= 1 << 3;
        } else {
            val &= !(1 << 3);
        }
        self.portpmsc.write(val);
    }

    pub fn best_effor_service_latency(&self) -> u8 {
        (unsafe { self.portpmsc.read() } >> 4 & 0xF) as u8
    }

    pub unsafe fn set_best_effort_service_latency(&mut self, status: u8) {
        let mut val = self.portpmsc.read();
        val &= !(0xF << 4);
        val |= (status as u32) << 4;
        self.portpmsc.write(val);
    }

    pub fn l1_device_slot(&self) -> u8 {
        (unsafe { self.portpmsc.read() } >> 8 & 0xFF) as u8
    }

    pub unsafe fn set_l1_device_slot(&mut self, status: u8) {
        let mut val = self.portpmsc.read();
        val &= !(0xFF << 8);
        val |= (status as u32) << 8;
        self.portpmsc.write(val);
    }

    pub fn hardware_lpm_enabled(&self) -> bool {
        unsafe { self.portpmsc.get_bit(16) }
    }

    pub unsafe fn set_hardware_lmp_enabled(&mut self, status: bool) {
        let mut val = self.portpmsc.read();
        if status {
            val |= 1 << 16;
        } else {
            val &= !(1 << 16);
        }
        self.portpmsc.write(val);
    }

    pub fn port_test_control(&self) -> u8 {
        (unsafe { self.portpmsc.read() } >> 28) as u8
    }

    pub unsafe fn set_port_test_control(&mut self, status: u8) {
        let mut val = self.portpmsc.read();
        val &= !(0xF << 28);
        val |= (status as u32) << 28;
        self.portpmsc.write(val);
    }

    // portli - usb3

    pub fn link_error_count(&self) -> u16 {
        (unsafe { self.portli.read() } & 0xFFFF) as u16
    }

    pub unsafe fn reset_link_error_count(&mut self) {
        let mut val = self.portli.read();
        val &= !0xFFFF;
        self.portli.write(val);
    }

    pub fn rx_lane_count(&self) -> u8 {
        (unsafe { self.portli.read() } >> 16 & 0xF) as u8
    }

    pub fn tx_lane_count(&self) -> u8 {
        (unsafe { self.portli.read() } >> 20 & 0xF) as u8
    }

    // NOTE: The documentation is not clear where
    // PORTEXSC is located, so no fuctionality bound
    // to that register is made available.

    // PORTHLPMC is currently not made available because
    // of lazyness.
}
