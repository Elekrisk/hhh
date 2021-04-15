use core::{cell::UnsafeCell, convert::{TryFrom, TryInto}, fmt::Debug, marker::PhantomData, mem::{Discriminant, discriminant}};

use num_traits::PrimInt;


#[repr(C)]
pub struct Register<T: PrimInt + BinWidth, O: RegisterValue<T>> {
    data: UnsafeCell<T>,
    _marker0: PhantomData<O>,
}

impl<T: PrimInt + BinWidth, O: RegisterValue<T>> Register<T, O> {
    fn _read(&self) -> T {
        unsafe {
            self.data.get().read_volatile()
        }
    }

    fn _write(&mut self, value: T) {
        unsafe {
            self.data.get().write_volatile(value);
        }
    }

    fn _modify<F: FnOnce(&mut T)>(&mut self, func: F) {
        unsafe {
            let mut value = self.data.get().read_volatile();
            func(&mut value);
            self.data.get().write_volatile(value);
        }
    }

    pub fn read(&self, dummy_value: &O) -> O {
        let val = self._read();
        Self::get(val, dummy_value)
    }

    pub fn write(&mut self, value: O) {
        self._modify(|v| Self::set(v, value));
    }

    fn get(val: T, dummy_value: &O) -> O {
        let mask_info = dummy_value.bits();
        O::from_value(mask_info.mask(val), dummy_value)
    }

    fn set(val: &mut T, value: O) {
        let mask_info = value.bits();
        let value = value.to_value();
        let value = value << mask_info.bit_offset as usize;
        let mask = !(mask_info.mask(!T::zero()) << mask_info.bit_offset as usize);
        let mut v = *val & mask;
        v = v | value;
        *val = v;
    }
}

pub struct MaskInfo {
    bit_size: u8,
    bit_offset: u8
}

impl MaskInfo {
    pub fn new(bit_size: u8, bit_offset: u8) -> Self {
        Self {
            bit_size,
            bit_offset
        }
    }

    fn mask<T: PrimInt + BinWidth>(&self, val: T) -> T {
        (val >> self.bit_offset as usize) & (!T::zero() >> (T::bin_width() - self.bit_size) as usize)
    }
}

pub trait RegisterValue<T: PrimInt + BinWidth>: Sized {
    fn from_value(val: T, dummy_value: &Self) -> Self;
    fn to_value(&self) -> T;
    /// Returns a MaskInfo which identifies which bits in the register that the value represents.
    fn bits(&self) -> MaskInfo;
}


pub trait BinWidth {
    fn bin_width() -> u8;
}

impl BinWidth for u8 {
    fn bin_width() -> u8 { 8 }
}

impl BinWidth for u16 {
    fn bin_width() -> u8 { 16 }
}

impl BinWidth for u32 {
    fn bin_width() -> u8 { 32 }
}

impl BinWidth for u64 {
    fn bin_width() -> u8 { 64 }
}