#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(asm)]
#![feature(assoc_char_funcs)]

mod spinlock;
mod writer;

use core::panic::PanicInfo;

use common::Framebuffer;

extern crate rlibc;

#[no_mangle]
pub extern "sysv64" fn _start(framebuffer: Framebuffer) {
    writer::init(framebuffer);
    writer::clear();
    writer::write_str("Hello, world!");
    x86_64::instructions::interrupts::int3();
    writer::write_str("after interrupt");

    loop {}
}

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    writer::write_str("Panic!\n");
    let loc = info.location().unwrap();
    writer::write_str(loc.file());
    writer::write_str(":");
    writer::write_u64(loc.line() as _);
    writer::write_str(":");
    writer::write_u64(loc.column() as _);
    writer::write_str(": ");
    writer::write_str(
        info.payload()
            .downcast_ref::<&'static str>()
            .unwrap_or(&"!no message!"),
    );
    loop {}
}

#[lang = "eh_personality"]
fn eh_personality() {}
