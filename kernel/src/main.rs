#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(asm)]
#![feature(assoc_char_funcs)]
#![feature(panic_info_message)]
#![feature(const_generics)]
#![feature(const_evaluatable_checked)]
#![feature(const_panic)]
#![feature(allocator_api)]
#![feature(const_mut_refs)]
#![feature(const_fn)]

mod usb;

#[macro_use]
extern crate common;

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
