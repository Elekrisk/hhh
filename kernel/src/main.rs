#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(asm)]
#![feature(panic_info_message)]
#![allow(incomplete_features)]
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
#![feature(const_option)]
#![feature(const_precise_live_drops)]

mod exceptions;
mod graphics;
mod idt;
mod pata;
mod pic;
mod ps2;
mod usb;

use graphics::{Pixel, Rect};
use pata::DiskSelect;
use ps2::keyboard::{self as keyboard, KeyCode, KeyState};

#[macro_use]
extern crate common;
#[macro_use]
extern crate alloc;

use alloc::prelude::v1::*;
use ps2::Ps2Driver;

use core::panic::PanicInfo;

use common::{Framebuffer, MachineInfo, MachineInfoC};
use x86_64::structures::{idt::InterruptStackFrame, paging::PageTable};

extern crate rlibc;
mod memory;

#[no_mangle]
pub extern "sysv64" fn _start(machine_info: MachineInfoC) -> ! {
    let machine_info: MachineInfo = machine_info.into();
    let page_table = x86_64::registers::control::Cr3::read()
        .0
        .start_address()
        .as_u64() as *mut PageTable;
    let page_table = unsafe { page_table.as_mut() }.unwrap();
    memory::init(page_table, machine_info.allocated_frames);

    unsafe { common::writer::init(machine_info.framebuffer) };
    common::writer::clear();

    unsafe {
        idt::initialize_idt();
    }

    // println!("xhci_base: {:x}", machine_info.xhci_base);

    // Enable interrupts
    unsafe { asm!("sti", options(nostack, nomem)) }

    // let driver = unsafe {
    //     usb::xhci::XhciDriver::new(machine_info.xhci_base as _)
    // };

    let mut ps2_driver = Ps2Driver::new();
    unsafe {
        pic::initialize();
        ps2_driver.initialize();
    }

    let mut first_sector = [0; 512];

    unsafe {
        pata::init();
        pata::read_sectors(DiskSelect::Master, 0, &mut first_sector).unwrap();
        println!("First sectors of master: {:x?}", first_sector);
    }


    // unsafe { graphics::init(machine_info.framebuffer); }

    // graphics::draw(|g| {
    //     g.draw_rect(Rect::new(10, 10, 500, 500), Pixel::new(128, 128, 128));
    //     g.draw_line(graphics::Line::VerticalLine{ x: 255, y: 20, length: 480 }, 4, Pixel::new(255, 255, 255));
    // });

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
                None => "Box<Any>",
            };
            println!("{}: Panic: '{}'", loc, msg);
        }
    }

    halt()
}

fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[lang = "eh_personality"]
fn eh_personality() {}
