use core::{cell::UnsafeCell, ops::{BitAnd, Shr}};

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


pub struct Register<T: Copy = u32> {
    data: UnsafeCell<T>
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


pub trait Num: Eq + Copy + Sized + Shr<usize, Output=Self> + BitAnd<Self, Output=Self> {
    const ONE: Self;
}

impl Num for u32 {
    const ONE: Self = 1;
}

impl Num for u64 {
    const ONE: Self = 1;
}
