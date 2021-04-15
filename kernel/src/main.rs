#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(asm)]
#![feature(panic_info_message)]
#![feature(const_generics)]
#![feature(const_evaluatable_checked)]
#![feature(const_panic)]
#![feature(allocator_api)]
#![feature(const_mut_refs)]
#![feature(const_fn)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_prelude)]
#![feature(default_alloc_error_handler)]
#![feature(inherent_associated_types)]

mod usb;
mod exceptions;

#[macro_use]
extern crate common;
extern crate alloc;

use alloc::prelude::v1::*;

use core::panic::PanicInfo;

use common::{Framebuffer, MachineInfo, MachineInfoC};
use x86_64::structures::paging::PageTable;

extern crate rlibc;
mod memory;

#[no_mangle]
pub extern "sysv64" fn _start(machine_info: MachineInfoC) {
    let machine_info: MachineInfo = machine_info.into();
    let page_table = x86_64::registers::control::Cr3::read().0.start_address().as_u64() as *mut PageTable;
    let page_table = unsafe { page_table.as_mut() }.unwrap();
    memory::init(page_table, machine_info.allocated_frames);

    unsafe { common::writer::init(machine_info.framebuffer) };
    common::writer::clear();

    println!("xhci_base: {:x}", machine_info.xhci_base);

    let mut idt = Box::new(x86_64::structures::idt::InterruptDescriptorTable::new());

    {
        use exceptions::*;
        idt.alignment_check.set_handler_fn(alignment_check);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded);
        idt.breakpoint.set_handler_fn(breakpoint);
        idt.debug.set_handler_fn(debug);
        idt.device_not_available.set_handler_fn(device_not_available);
        idt.divide_error.set_handler_fn(divide_error);
        idt.double_fault.set_handler_fn(double_fault);
        idt.general_protection_fault.set_handler_fn(general_protection_fault);
        idt.invalid_opcode.set_handler_fn(invalid_opcode);
        idt.invalid_tss.set_handler_fn(invalid_tss);
        idt.machine_check.set_handler_fn(machine_check);
        idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt);
        idt.overflow.set_handler_fn(overflow);
        idt.page_fault.set_handler_fn(page_fault);
        idt.security_exception.set_handler_fn(security_exception);
        idt.segment_not_present.set_handler_fn(segment_not_present);
        idt.simd_floating_point.set_handler_fn(simd_floating_point);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault);
        idt.virtualization.set_handler_fn(virtualization);
        idt.x87_floating_point.set_handler_fn(x87_floating_point);
    }

    unsafe { idt.load_unsafe(); }

    let driver = unsafe {
        usb::xhci::XhciDriver::new(machine_info.xhci_base as _)
    };

    loop {}
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    let loc = info.location().unwrap();
    match info.message() {
        Some(v) => println!("{}: Panic: '{}'", loc, v),
        None => {
            let msg = match info.payload().downcast_ref::<&'static str>() {
                Some(v) => *v,
                None => "Box<Any>"
            };
            println!("{}: Panic: '{}'", loc, msg);
        }
    }

    loop {}
}

#[lang = "eh_personality"]
fn eh_personality() {}
