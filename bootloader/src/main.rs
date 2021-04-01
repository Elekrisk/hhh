#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(abi_efiapi)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_array_assume_init)]

extern crate rlibc;
extern crate uefi;
extern crate uefi_services;

mod elf;

use alloc::vec::Vec;
use common::Framebuffer;
use elf::{Elf, EntryType, HeaderEntry, SectionType};
use uefi::{prelude::*, proto::{console::gop::GraphicsOutput, media::{file::{File, FileAttribute, FileMode, FileType}, fs::SimpleFileSystem}}, table::boot::{AllocateType, MemoryType}};
use uefi::{Handle, Status, table::{Boot, SystemTable}};
use log::info;
extern crate alloc;

fn to_utf16<const CHAR_COUNT: usize>(string: &str) -> [u16; CHAR_COUNT] {
    let mut ret = [0; CHAR_COUNT];
    ucs2::encode(string, &mut ret).unwrap();
    ret
}

#[entry]
fn efi_main(image_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&system_table).unwrap().unwrap();
    let (entry, framebuffer) = {
        system_table.stdout().clear().unwrap().unwrap();

        info!("Bootloader initialized");

        let fs_proto = system_table.boot_services().locate_protocol::<SimpleFileSystem>().unwrap().unwrap();
        // Safety: We override the usafe cell and thus have only one reference.
        // This reference should never be used after another call to locate_protocol::<SimpleFileSystem>().
        let fs_proto = unsafe { fs_proto.get().as_mut().unwrap() };
        let mut root_dir = fs_proto.open_volume().unwrap_success();
        let mut buffer = Vec::new();
        let kernel_file_info = loop {
            let buffer_size = root_dir.read_entry(&mut []).unwrap_err().data().unwrap();
            buffer.resize(buffer_size, 0);
            let entries = match root_dir.read_entry(&mut buffer).unwrap_success() {
                Some(v) => v,
                None => panic!("kernel.elf not found")
            };
            if entries.attribute().contains(FileAttribute::DIRECTORY) {
                info!("directory {}", entries.file_name());
                continue;
            }
            info!("file {}", entries.file_name());
            if entries.file_name().to_u16_slice() == &to_utf16::<10>("kernel.elf") {
                break entries;
            }
        };

        let kernel_size = kernel_file_info.file_size();
        let kernel_file = root_dir.open("kernel.elf", FileMode::Read, FileAttribute::empty()).unwrap_success();
        
        let mut elf_buffer = Vec::with_capacity(kernel_size as _);
        elf_buffer.resize(kernel_size as _, 0);
        let kernel_elf = match kernel_file.into_type().unwrap_success() {
            FileType::Regular(mut f) => {
                if f.read(&mut elf_buffer).unwrap_success() < kernel_size as _ {
                    log::error!("Entire kernel file was not read at once");
                    loop {}
                }
                Elf::parse(&elf_buffer).unwrap()
            },
            FileType::Dir(_) => unreachable!()
        };

        info!("Program header count: {}", kernel_elf.program_headers.len());
        info!("Section header count: {}", kernel_elf.section_headers.len());
        
        let mut to_allocate = Vec::new();

        for segment in &kernel_elf.program_headers {
            if segment.entry_type == EntryType::Load {
                let start = segment.virtual_addr;
                let end = start + segment.mem_size;
                let mut align_mask = !0;
                let mut t = segment.align;
                while t > 1 {
                    align_mask <<= 1;
                    t >>= 1;
                }
                let aligned_start = start & align_mask;
                let first_page = aligned_start >> 12; // 4K pages
                let last_page = end >> 12;

                to_allocate.push((first_page, last_page));

                let mut i = 0;
                while i < to_allocate.len() {
                    let mut j = i + 1;
                    while j < to_allocate.len() {
                        let (a1, b1) = to_allocate[i];
                        let (a2, b2) = to_allocate[j];

                        if a1 <= a2 && b1 >= a2 || a1 <= b2 && b1 >= b2 || a1 >= a2 && b1 <= b2 {
                            let first = a1.max(a2);
                            let last = b1.max(b2);
                            to_allocate.remove(j);
                            to_allocate.remove(i);
                            to_allocate.push((first, last));
                        }
                        j += 1;
                    }
                    i += 1;
                }
            }
        }

        info!("Page offset ranges to allocate (inclusive):");
        for (start, end) in &to_allocate {
            info!("{} .. {}", start, end);
        }

        let offset = 0x400000;
        let offset_page = offset >> 12;
        info!("Actual pages to allocate:");
        for (start, end) in &mut to_allocate {
            *start += offset_page;
            *end += offset_page;
            info!("{} .. {}", *start, *end);
        }

        for (start, end) in &to_allocate {
            let page_count = *end - *start + 1;
            system_table.boot_services().allocate_pages(AllocateType::Address((*start as usize) << 12), MemoryType::LOADER_DATA, page_count as _).unwrap().unwrap();

            // Safety:
            // - enough memory should have been allocated just above to contain buffer..buffer+size
            // - size should be exactly how many bytes have been allocated
            unsafe { system_table.boot_services().memset(((*start as usize) << 12) as *mut u8, page_count as usize * 4096, 0); }
        }

        for segment in &kernel_elf.program_headers {
            if segment.entry_type == EntryType::Load {
                let start = segment.virtual_addr;
                let actual_start = start + offset;
                let mut align_mask = !0;
                let mut t = segment.align;
                while t > 1 {
                    align_mask <<= 1;
                    t >>= 1;
                }
                info!("Loading segment to {:x}", actual_start);
                
                // Safety:
                // - src is trivially valid for data.len() bytes
                // - dst should be valid for data.len() bytes, as enough pages should have been allocated to contain it
                unsafe { core::ptr::copy(segment.data.as_ptr(), actual_start as *mut u8, segment.data.len()); }
            }
        }
        info!("Entry is at {:x}", kernel_elf.entry + offset);

        let framebuffer = system_table.boot_services().locate_protocol::<GraphicsOutput>().unwrap().unwrap();
        let framebuffer = unsafe {framebuffer.get().as_mut().unwrap() };
        let current_mode = framebuffer.current_mode_info();
        let pixel_format = current_mode.pixel_format();
        info!("pixel format: {:?}", pixel_format);
        let framebuffer = Framebuffer {
            ptr: framebuffer.frame_buffer().as_mut_ptr(),
            resolution_x: current_mode.resolution().0,
            resolution_y: current_mode.resolution().1,
            stride: current_mode.stride()
        };

        
        let actual_entry = offset + kernel_elf.entry;
        
        info!("Framebuffer: {:x}", framebuffer.ptr as u64);
        info!("Kernel should have been loaded into memory.");
        info!("First 8 bytes after jump are:");
        unsafe {
            let base = actual_entry as *const u8;
            let a = base.read();
            let b = base.add(1).read();
            let c = base.add(2).read();
            let d = base.add(3).read();
            let e = base.add(4).read();
            let f = base.add(5).read();
            let g = base.add(6).read();
            let h = base.add(7).read();
            info!("{:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x}", a, b, c, d, e, f, g, h);
        };
        
        system_table.boot_services().wait_for_event(&mut [system_table.stdin().wait_for_key_event()]).unwrap_success();

        info!("Jump!");
        
        (unsafe { core::mem::transmute::<_, extern "sysv64" fn(Framebuffer)>(actual_entry) }, framebuffer)
    };
    
    
    let memmapsize = system_table.boot_services().memory_map_size();
    let mut memmapbuffer = Vec::with_capacity(memmapsize + 128);
    memmapbuffer.resize(memmapsize + 128, 0);
    system_table.exit_boot_services(image_handle, &mut memmapbuffer).unwrap_success();
    
    entry(framebuffer);

    loop {}
}
