#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(asm)]
#![feature(assoc_char_funcs)]
#![feature(panic_info_message)]
#![feature(const_generics)]
#![feature(const_evaluatable_checked)]
#![feature(const_panic)]

mod usb;

#[macro_use]
extern crate common;

use core::panic::PanicInfo;

use common::{Framebuffer, MachineInfo};

extern crate rlibc;

#[no_mangle]
pub extern "sysv64" fn _start(machine_info: MachineInfo) {
    unsafe { common::writer::init(machine_info.framebuffer) };
    common::writer::clear();
    // x86_64::instructions::interrupts::int3();
    println!("Hello, world!");

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
