#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(asm)]

mod spinlock;
mod writer;

use core::panic::PanicInfo;

use common::Framebuffer;

extern crate rlibc;

#[no_mangle]
pub extern "sysv64" fn _start(framebuffer: Framebuffer) {
    unsafe {
        writer::init(framebuffer);
        writer::clear();
        writer::write_str("Hello from kernel :D");
    }

    loop{}
}

#[panic_handler]
fn panic_handler(_: &PanicInfo) -> ! {
    unsafe { writer::write_str("Panic!"); }
    loop {}
}

#[lang = "eh_personality"]
fn eh_personality() {

}