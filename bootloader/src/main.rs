#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(abi_efiapi)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(asm)]
#![feature(vec_into_raw_parts)]
#![feature(abi_x86_interrupt)]
#![feature(const_generics)]
#![feature(const_evaluatable_checked)]
#![feature(panic_info_message)]

extern crate rlibc;
extern crate uefi;
extern crate uefi_services;

#[macro_use]
extern crate common;

mod elf;
mod exceptions;
mod panic;

use acpi::{AcpiHandler, PhysicalMapping, mcfg::{Mcfg, McfgEntry}, sdt::Signature};
use alloc::vec::Vec;
use common::{Framebuffer, MachineInfo, MachineInfoC};
use elf::{Elf, EntryType, HeaderEntry, SectionType};
use exceptions::page_fault;
use uefi::{prelude::*, proto::{console::gop::GraphicsOutput, media::{file::{File, FileAttribute, FileMode, FileType}, fs::SimpleFileSystem}}, table::{boot::{AllocateType, MemoryAttribute, MemoryDescriptor, MemoryType}, cfg::{self, ACPI2_GUID}}};
use uefi::{Handle, Status, table::{Boot, SystemTable}};
use log::info;
use x86_64::{PhysAddr, VirtAddr, instructions, structures::{gdt::{Descriptor, GlobalDescriptorTable}, idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode}, paging::{FrameAllocator, FrameDeallocator, OffsetPageTable, Page, PageSize, PageTable, PageTableFlags, Size1GiB, Size2MiB, Size4KiB, frame::{self, PhysFrame}, mapper::UnmapError, page_table::PageTableEntry}}};
use core::fmt::Debug;
extern crate alloc;

const _1G: u64 = 1 * 1024 * 1024 * 1024;
const _2M: u64 = 2 *        1024 * 1024;
const _4K: u64 = 4 *               1024;

fn to_utf16<const CHAR_COUNT: usize>(string: &str) -> [u16; CHAR_COUNT] {
    let mut ret = [0; CHAR_COUNT];
    ucs2::encode(string, &mut ret).unwrap();
    ret
}

#[entry]
fn efi_main(image_handle: Handle, st: SystemTable<Boot>) -> Status {
    uefi_services::init(&st).unwrap().unwrap();
    let (entry, mut machine_info, kernel_addresses) = {
        st.stdout().clear().unwrap().unwrap();
        let framebuffer = st.boot_services().locate_protocol::<GraphicsOutput>().unwrap().unwrap();
        let framebuffer = unsafe {framebuffer.get().as_mut().unwrap() };
        let current_mode = framebuffer.current_mode_info();
        let pixel_format = current_mode.pixel_format();
        let framebuffer = Framebuffer {
            ptr: framebuffer.frame_buffer().as_mut_ptr() as _,
            resolution_x: current_mode.resolution().0,
            resolution_y: current_mode.resolution().1,
            stride: current_mode.stride()
        };
        unsafe {
            common::writer::init(framebuffer.clone());
        }
        println!("Writer initialized");

        #[derive(Copy, Clone)]
        struct SimpleHandler;
        impl AcpiHandler for SimpleHandler {
            unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> PhysicalMapping<Self, T> {
                let ptr = core::ptr::NonNull::new(physical_address as *mut T).unwrap();
                PhysicalMapping {
                    physical_start: physical_address,
                    virtual_start: ptr,
                    region_length: size,
                    mapped_length: size,
                    handler: *self
                }
            }

            fn unmap_physical_region<T>(&self, region: &PhysicalMapping<Self, T>) {
                
            }
        }

        println!("Finding ACPI tables...");
        let mut acpi_tables = None;
        for config_table in st.config_table() {
            if config_table.guid == ACPI2_GUID {
                unsafe {
                    acpi_tables = Some(acpi::AcpiTables::from_rsdp(SimpleHandler, config_table.address as usize).unwrap());
                };
            }
        }
        let acpi_tables = acpi_tables.unwrap();

        fn entries(mcfg: &Mcfg) -> &[McfgEntry] {
            use acpi::AcpiTable;
            use core::mem;
            use core::slice;
            let length = mcfg.header().length as usize - mem::size_of::<Mcfg>();

            // Intentionally round down in case length isn't an exact multiple of McfgEntry size
            // (see rust-osdev/acpi#58)
            let num_entries = length / mem::size_of::<McfgEntry>();

            unsafe {
                let pointer =
                    (mcfg as *const Mcfg as *const u8).offset(mem::size_of::<Mcfg>() as isize) as *const McfgEntry;
                slice::from_raw_parts(pointer, num_entries)
            }
        }

        #[derive(Clone, Copy, Debug)]
        #[repr(C, packed)]
        pub struct McfgEntry {
            base_address: u64,
            pci_segment_group: u16,
            bus_number_start: u8,
            bus_number_end: u8,
            _reserved: u32,
        }

        let xhci_base = if let Some(mcfg) = unsafe { acpi_tables.get_sdt::<Mcfg>(Signature::MCFG).unwrap() } {
            let mcfg = unsafe { mcfg.virtual_start.as_ref() };
            let entries = entries(mcfg);
            println!("MCFG entry count: {}", entries.len());
            for entry in entries {
                println!("bus 0x{:x}..=0x{:x}, addr {:x}", entry.bus_number_start, entry.bus_number_end, entry.base_address);
            }
            let base = entries[0].base_address;
            
            let mut xhci_cfg_base = None;
            for i in 0.. {
                let base = base + (i << 12);
                if base > 0xffffffff { break; } 
                let base = base as *const u32;
                let info = unsafe { base.add(2).read() };
                if info != !0u32 && info != 0 {
                    let class = info >> 24 & 0xFF;
                    let subclass = info >> 16 & 0xFF;
                    let function = info >> 8 & 0xFF;
                    let revision = info & 0xFF;
                    if class == 0xC && subclass == 0x3 && function == 0x30 {
                        xhci_cfg_base = Some(base);
                        break;
                    }
                }
            }
            let xhci_cfg_base = xhci_cfg_base.expect("No USB 3 controller found");
            
            unsafe {
                let bar0 = xhci_cfg_base.add(4).read() as u64;
                let bar1 = xhci_cfg_base.add(5).read() as u64;
                bar1 << 32 | bar0
            }
        } else {
            println!("PCIe not supported");
            
            fn set_cfg_addr(addr: u32) {
                unsafe {
                    asm!(
                        "mov dx, 0xCF8",
                        "out dx, eax",
                        in("eax") addr,
                        lateout("dx") _,
                        options(nostack)
                    );
                }
            }

            fn read_data() -> u32 {
                unsafe {
                    let mut out;
                    asm!(
                        "mov dx, 0xCFC",
                        "in eax, dx",
                        out("eax") out,
                        lateout("dx") _,
                        options(nostack)
                    );
                    out
                }
            }
            fn write_data(data: u32) {
                unsafe {
                    asm!(
                        "mov dx, 0xCFC",
                        "out dx, eax",
                        in("eax") data,
                        lateout("dx") _,
                        options(nostack)
                    );
                }
            }

            let bus = 0xC;
            let slot = 0x3;
            let func = 0x30;

            let mut xhci_base = None;
            for bus in 0..256 {
                for device in 0..32 {
                    for function in 0..8 {
                        let addr = 0x80000000 | bus << 16 | device << 11 | function << 8;
                        set_cfg_addr(addr);
                        let data = read_data();
                        if data != 0xFFFFFFFF {
                            // Device found!
                            set_cfg_addr(addr | 0x8);
                            let data = read_data();
                            let class = data >> 24 & 0xFF;
                            let subclass = data >> 16 & 0xFF;
                            let function = data >> 8 & 0xFF;
                            if class == 0xC && subclass == 0x3 && function == 0x30 {
                                // Found XHCI!
                                set_cfg_addr(addr | 0x10);
                                let bar0 = read_data() as u64;
                                let bar0 = bar0 & !0xFFF;
                                set_cfg_addr(addr | 0x14);
                                let bar1 = read_data() as u64;
                                xhci_base = Some(bar1 << 32 | bar0);
                                println!("xhci_base: {:x}", xhci_base.unwrap());
                                // loop {}
                            }
                        }
                    }
                }
            }
            xhci_base.expect("No USB 3 controller found")
        };
        
        
        // let fs_proto = system_table.boot_services().locate_protocol::<SimpleFileSystem>().unwrap().unwrap();
        let fs_proto = st.boot_services().get_image_file_system(image_handle).unwrap().unwrap();
        
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
                println!("directory {}", entries.file_name());
                continue;
            }
            println!("file {}", entries.file_name());
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
        
        println!("Program header count: {}", kernel_elf.program_headers.len());
        println!("Section header count: {}", kernel_elf.section_headers.len());
        
        let mut lowest_base = u64::MAX;
        for segment in &kernel_elf.program_headers {
            if segment.entry_type == EntryType::Load {
                lowest_base = lowest_base.min(segment.virtual_addr);
            }
        }
        
        let mut kernel_addresses = Vec::new();
        
        for segment in &kernel_elf.program_headers {
            if segment.entry_type == EntryType::Load {
                let virt_start = segment.virtual_addr;
                let virt_end = virt_start + segment.mem_size;
                let phys_start = segment.virtual_addr - lowest_base;
                let phys_end = phys_start + segment.mem_size;
                let mut align_mask = !0;
                let mut t = segment.align;
                while t > 1 {
                    align_mask <<= 1;
                    t >>= 1;
                }
                let aligned_virt_start = virt_start & align_mask;
                let first_virt_page = aligned_virt_start >> 21; // 2M pages
                let last_virt_page = virt_end >> 21;
                let aligned_phys_start = phys_start & align_mask;
                let first_phys_page = aligned_phys_start >> 21; // 2M pages
                let last_phys_page = phys_end >> 21;
                
                kernel_addresses.push(((first_virt_page, last_virt_page), (first_phys_page, last_phys_page)));
            }
        }

        let mut has_changed = true;
        while has_changed {
            has_changed = false;
            let mut i = 0;
            while i < kernel_addresses.len() {
                let mut j = i + 1;
                while j < kernel_addresses.len() {
                    let ((vstart1, vend1), (pstart1, pend1)) = kernel_addresses[i];
                    let ((vstart2, vend2), (pstart2, pend2)) = kernel_addresses[j];
                    
                    // Possible cases:
                    //  s-------e
                    //      s------
                    //
                    //  s-------e
                    // -----e
                    //
                    //   s------e
                    // s-----------e
                    //
                    //  s-------e
                    //              s-----e
                    //
                    // We assume vstart1 + C = pstart1 etc for all v.. and p.. pairs where C is constant
                    // As such, comparisons between v.. will also hold for p..
                    if  // s------e
                    //    s--
                    vstart1 <= vstart2 && vend1 >= vstart2 ||
                    // s-----e
                    // ---e
                    vstart1 <= vend2 && vend1 >= vend2 ||
                    // s-----e
                    //---------
                    vstart1 >= vstart2 && vend1 <= vend2 {
                        let vfirst = vstart1.min(vstart2);
                        let pfirst = pstart1.min(pstart2);
                        let vlast = vend1.max(vend2);
                        let plast = pend1.max(pend2);
                        kernel_addresses.remove(j);
                        kernel_addresses.remove(i);
                        kernel_addresses.push(((vfirst, vlast), (pfirst, plast)));
                        has_changed = true;
                        continue;
                    }
                    j += 1;
                }
                i += 1;
            }
        }
        
        println!("2M page offset ranges to allocate (inclusive):");
        for (_, (pstart, pend)) in &kernel_addresses {
            println!("{} .. {}", pstart, pend);
        }
        
        let offset = 0x40_00_00;
        let offset_page = offset >> 21;
        println!("Actual 2M pages to allocate:");
        for (_, (pstart, pend)) in &mut kernel_addresses {
            *pstart += offset_page;
            *pend += offset_page;
            println!("{} .. {}", *pstart, *pend);
        }
        
        for (_ ,(pstart, pend)) in &kernel_addresses {
            let page_count = *pend - *pstart + 1;
            st.boot_services().allocate_pages(AllocateType::Address((*pstart as usize) << 21), MemoryType::LOADER_DATA, (page_count as usize) * 512 /* 2M to 4K pages */).unwrap().unwrap();
            
            // Safety:
            // - enough memory should have been allocated just above to contain buffer..buffer+size
            // - size should be exactly how many bytes have been allocated
            unsafe { st.boot_services().memset(((*pstart as usize) << 21) as *mut u8, page_count as usize * 2*1024*1024 /* 2M pages */, 0); }
        }
        
        for segment in &kernel_elf.program_headers {
            if segment.entry_type == EntryType::Load {
                let start = segment.virtual_addr;
                let actual_start = start + offset - lowest_base;
                let mut align_mask = !0;
                let mut t = segment.align;
                while t > 1 {
                    align_mask <<= 1;
                    t >>= 1;
                }
                println!("Loading segment to {:x}", actual_start);
                
                // Safety:
                // - src is trivially valid for data.len() bytes
                // - dst should be valid for data.len() bytes, as enough pages should have been allocated to contain it
                unsafe { core::ptr::copy(segment.data.as_ptr(), actual_start as *mut u8, segment.data.len()); }
            }
        }
        let phys_entry = kernel_elf.entry + offset - lowest_base;
        let virt_entry = kernel_elf.entry;
        println!("Entry is at phys {:x} virt {:x}", phys_entry, virt_entry);
        
        
        
        println!("Framebuffer: {:x}", framebuffer.ptr as u64);
        println!("Kernel should have been loaded into memory.");
        println!("First 8 bytes after jump (phys) are:");
        unsafe {
            let base = phys_entry as *const u8;
            let a = base.read();
            let b = base.add(1).read();
            let c = base.add(2).read();
            let d = base.add(3).read();
            let e = base.add(4).read();
            let f = base.add(5).read();
            let g = base.add(6).read();
            let h = base.add(7).read();
            println!("{:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x}", a, b, c, d, e, f, g, h);
        };
        
        // println!("Press any key to view memmap");
        // wait_for_key(&st);
        
        let memory_map_size = st.boot_services().memory_map_size();
        let mut memory_map_buffer = Vec::new();
        memory_map_buffer.resize(memory_map_size + 256, 0);
        let mut max_physical = 0;
        for entry in st.boot_services().memory_map(&mut memory_map_buffer).unwrap().unwrap().1 {
            println!("{:?} phys {:x} virt {:x} page_count {}", entry.ty, entry.phys_start, entry.virt_start, entry.page_count);
            if entry.phys_start + entry.page_count * 4096 > max_physical {
                max_physical = entry.phys_start + entry.page_count * 4096;
            }
        }

        let frame_count = max_physical / 4096;
        let allocated_frames = alloc::vec![0; (frame_count / 8) as usize];
        
        // println!("Press any key to continue");
        // wait_for_key(&st);
        // #[cfg(feature = "wait_for_gdb")]
        // println!("Will wait for GDB after jump");
        // println!("Press any key to jump to kernel");
        // wait_for_key(&system_table);
        
        // println!("Jump!");

        let machine_info = MachineInfo {
            framebuffer,
            xhci_base,
            allocated_frames: allocated_frames.leak()
        };

        (unsafe { core::mem::transmute::<_, extern "sysv64" fn(MachineInfoC)>(virt_entry) }, machine_info, kernel_addresses.leak())
    };
    
    let memmapsize = st.boot_services().memory_map_size();
    let desc_size = core::mem::size_of::<MemoryDescriptor>();
    let vec_size = memmapsize + desc_size*2;
    let mut memmapbuffer = Vec::with_capacity(vec_size);
    let mut memmap: Vec<MemoryDescriptor> = Vec::with_capacity(vec_size/core::mem::size_of::<MemoryDescriptor>());
    memmapbuffer.resize(vec_size, 0);
    let st = {
        let (st, memmap_iter) = st.exit_boot_services(image_handle, &mut memmapbuffer).unwrap_success();
        memmap.extend(memmap_iter);
        st
    };

    memmap.sort_unstable_by_key(|m| m.phys_start);

    unsafe {
        let code_segment = GDT.add_entry(Descriptor::kernel_code_segment());
        let data_segment = GDT.add_entry(Descriptor::kernel_data_segment());
        GDT.load();
        instructions::segmentation::load_ss(data_segment);
        instructions::segmentation::set_cs(code_segment);

        {
            use exceptions::*;
            IDT.alignment_check.set_handler_fn(alignment_check);
            IDT.bound_range_exceeded.set_handler_fn(bound_range_exceeded);
            IDT.debug.set_handler_fn(debug);
            IDT.device_not_available.set_handler_fn(device_not_available);
            IDT.divide_error.set_handler_fn(divide_error);
            IDT.general_protection_fault.set_handler_fn(general_protection_fault);
            IDT.invalid_opcode.set_handler_fn(invalid_opcode);
            IDT.invalid_tss.set_handler_fn(invalid_tss);
            IDT.machine_check.set_handler_fn(machine_check);
            IDT.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt);
            IDT.overflow.set_handler_fn(overflow);
            IDT.security_exception.set_handler_fn(security_exception);
            IDT.segment_not_present.set_handler_fn(segment_not_present);
            IDT.simd_floating_point.set_handler_fn(simd_floating_point);
            IDT.stack_segment_fault.set_handler_fn(stack_segment_fault);
            IDT.virtualization.set_handler_fn(virtualization);
            IDT.x87_floating_point.set_handler_fn(x87_floating_point);
            IDT.breakpoint.set_handler_fn(breakpoint);
            IDT.page_fault.set_handler_fn(page_fault);
            IDT.double_fault.set_handler_fn(double_fault);
        }

        IDT.load();
        
        let pml4t = (PAGE_ALLOCATOR.allocate_frame().unwrap().start_address().as_u64() as *mut PageTable).as_mut().unwrap();
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        
        {
            let pdpt_addr = PAGE_ALLOCATOR.allocate_frame().unwrap().start_address();
            pml4t[511].set_addr(pdpt_addr, flags);
            let pdpt = (pdpt_addr.as_u64() as *mut PageTable).as_mut().unwrap();

            let mut physical_cap = memmap.iter().map(|m| {
                if m.phys_start + m.page_count * _4K > 16*_1G { println!("mem of type {:?}: {:x}", m.ty, m.phys_start); }
                m.phys_start + m.page_count * 4096
            }).max().unwrap();
            if machine_info.framebuffer.ptr as u64 > physical_cap {
                println!("Framebuffer is above physical cap, extending physical cap...");
                let framebuffer = &machine_info.framebuffer;
                physical_cap = framebuffer.ptr as u64 + framebuffer.resolution_y as u64 * framebuffer.stride as u64;
            }
            println!("Physical cap: 0x{:x} : {} B, {} KB, {} MB, {} GB", physical_cap, physical_cap, physical_cap >> 10, physical_cap >> 20, physical_cap >> 30);
            let gig_pages = (physical_cap + _1G - 1) / _1G;
            println!("1G pages: {}", gig_pages);

            for (i, entry) in pdpt.iter_mut().take(gig_pages as _).enumerate() {
                let pdt_addr = PAGE_ALLOCATOR.allocate_frame().unwrap().start_address();
                entry.set_addr(pdt_addr, flags);
                let pdt = (pdt_addr.as_u64() as *mut PageTable).as_mut().unwrap();
                for (j, entry) in pdt.iter_mut().enumerate() {
                    entry.set_addr(PhysAddr::new(idx2virt(0, i as _, j as _, 0).as_u64()), flags | PageTableFlags::HUGE_PAGE);
                }
            }

            println!("mapping used memory");
            for entry in &memmap {
                match entry.ty {
                    MemoryType::CONVENTIONAL |
                    MemoryType::RESERVED |
                    MemoryType::UNUSABLE => {},
                    _ => {
                        let start = entry.phys_start;
                        let page_count = entry.page_count;
                        println!("Entry to map: {:x}..{:x}", start, start+(page_count<<12));
                        let start_page = start >> 12;
                        for page in start_page..start_page + page_count {
                            map(pml4t, VirtAddr::new(page << 12), PhysFrame::from_start_address(PhysAddr::new(page << 12)).unwrap()).unwrap();
                            machine_info.allocated_frames[(page / 8) as usize] |= 1 << (page % 8);
                        }
                    }
                }
            }

            println!("maping kernel");
            for ((vstart, vend), (pstart, pend)) in kernel_addresses {
                for (vpage, ppage) in (*vstart..=*vend).zip(*pstart..=*pend) {
                    let idx4 = (vpage >> 18) as usize & 0x1FF;
                    let idx3 = (vpage >> 9) as usize & 0x1FF;
                    let idx2 = vpage as usize & 0x1FF;
                    if pml4t[idx4].is_unused() {
                        let pdpt_addr = PAGE_ALLOCATOR.allocate_frame().unwrap().start_address();
                        pml4t[idx4].set_addr(pdpt_addr, flags);
                    }
                    let pdpt = pml4t[idx4].as_page_table_mut().unwrap();

                    println!("mapping {:x} -> {:x}", vpage<<21, ppage<<21);

                    if pdpt[idx3].is_unused() {
                        let pdpt_addr = PAGE_ALLOCATOR.allocate_frame().unwrap().start_address();
                        pdpt[idx3].set_addr(pdpt_addr, flags);
                    }
                    let pdt = pdpt[idx3].as_page_table_mut().unwrap();

                    if !pdt[idx2].is_unused() {
                        panic!("Tried mapping to already mapped page: {:x}", vstart);
                    }

                    pdt[idx2].set_addr(PhysAddr::new(ppage * _2M), flags | PageTableFlags::HUGE_PAGE);
                }
            }
            println!("done mapping kernel");
        }

        for entry in &mut memmap {
            entry.virt_start = idx2virt(511, 0, 0, 0).as_u64() | entry.phys_start;
        }

        st.runtime_services().set_virtual_address_map(&mut memmap).unwrap().unwrap();

        assert_mapped(pml4t, VirtAddr::new(entry as u64), true);
        assert_mapped(pml4t, VirtAddr::new(&GDT as *const GlobalDescriptorTable as u64), true);
        assert_mapped(pml4t, VirtAddr::new(&IDT as *const InterruptDescriptorTable as u64), true);
        assert_mapped(pml4t, VirtAddr::new(page_fault as u64), true);
        let mut sp;
        asm!("mov {}, rsp", lateout(reg) sp);
        assert_mapped(pml4t, VirtAddr::new(sp), true);
        assert_mapped(pml4t, VirtAddr::new(machine_info.framebuffer.ptr as u64 | idx2virt(511, 0, 0, 0).as_u64()), true);


        let addr = PhysAddr::new((&PAGE_TABLE_4) as *const PageTable as u64);
        println!("address of page table: {:x}", addr.as_u64());
        // let frame: PhysFrame<Size4KiB> = PhysFrame::from_start_address(addr).unwrap();
        // writer::write_str("\n");

        println!("cr4: {:x}", x86_64::registers::control::Cr4::read_raw());
        println!("current cr3: {:x}", x86_64::registers::control::Cr3::read().0.start_address().as_u64());

        let frame = PhysFrame::<Size4KiB>::from_start_address(PhysAddr::new(pml4t as *mut _ as u64)).unwrap();
        let old_table = (x86_64::registers::control::Cr3::read().0.start_address().as_u64() as *mut PageTable).as_mut().unwrap();
        // pml4t[0] = old_table[0].clone();
        // let frame = PhysFrame::<Size4KiB>::from_start_address(PhysAddr::new(old_table as *mut _ as u64)).unwrap();

        println!("flags of old pml4e0.pdpe0.pde0: {:?}", old_table[0].as_page_table().unwrap()[0].as_page_table().unwrap()[0].flags());
        println!("flags of new pml4e0.pdpe0: {:?}", pml4t[0].as_page_table().unwrap()[0].flags());

        println!("aaaa");
        machine_info.framebuffer.ptr = (machine_info.framebuffer.ptr as u64 | idx2virt(511, 0, 0, 0).as_u64()) as _;
        common::writer::update_ptr(machine_info.framebuffer.ptr);
        x86_64::registers::control::Cr3::write(frame, x86_64::registers::control::Cr3Flags::empty());
        println!("Loading new page table succeeded");
    }

    let machine_info = machine_info.into();

    wait_debug();

    // loop{}

    entry(machine_info);

    println!("DONE");

    loop {}
}

unsafe fn map(pml4: &mut PageTable, virt: VirtAddr, frame: PhysFrame) -> Result<(), &'static str> {
    let (idx4, idx3, idx2, idx1) = virt2idx(virt);
    
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    if pml4[idx4].is_unused() {
        let pdp_addr = PAGE_ALLOCATOR.allocate_frame().unwrap().start_address();
        pml4[idx4].set_addr(pdp_addr, flags);
    }
    let pdp = pml4[idx4].as_page_table_mut().unwrap();

    if pdp[idx3].is_unused() {
        let pd_addr = PAGE_ALLOCATOR.allocate_frame().unwrap().start_address();
        pdp[idx3].set_addr(pd_addr, flags);
    } else if pdp[idx3].flags().contains(PageTableFlags::HUGE_PAGE) {
        return Err("Trying to map to pdp entry containing a 1G page");
    }
    let pd = pdp[idx3].as_page_table_mut().unwrap();
    
    if pd[idx2].is_unused() {
        let pt_addr = PAGE_ALLOCATOR.allocate_frame().unwrap().start_address();
        pd[idx2].set_addr(pt_addr, flags);
    } else if pd[idx2].flags().contains(PageTableFlags::HUGE_PAGE) {
        return Err("Trying to map to pdp entry containing a 2M page");
    }
    let pt = pd[idx2].as_page_table_mut().unwrap();

    if pt[idx1].is_unused() {
        pt[idx1].set_frame(frame, flags);
    } else {
        return Err("Trying to map to already existing page");
    }

    Ok(())
}

trait AsPageTable {
    unsafe fn as_page_table(&self) -> Option<&PageTable>;
    unsafe fn as_page_table_mut(&mut self) -> Option<&mut PageTable>;
}

impl AsPageTable for PageTableEntry {
    unsafe fn as_page_table(&self) -> Option<&PageTable> {
        if !self.is_unused() && self.flags().contains(PageTableFlags::PRESENT) && !self.flags().contains(PageTableFlags::HUGE_PAGE) {
            Some((self.addr().as_u64() as *const PageTable).as_ref().unwrap())
        } else {
            None
        }
    }

    unsafe fn as_page_table_mut(&mut self) -> Option<&mut PageTable> {
        if !self.is_unused() && self.flags().contains(PageTableFlags::PRESENT) && !self.flags().contains(PageTableFlags::HUGE_PAGE) {
            Some((self.addr().as_u64() as *mut PageTable).as_mut().unwrap())
        } else {
            None
        }
    }
}

static mut PAGE_TABLE_4: PageTable = PageTable::new();

fn idx2virt(i4: usize, i3: usize, i2: usize, i1: usize) -> VirtAddr {
    let addr = ((i1 as u64) << 12) | ((i2 as u64) << 21) | ((i3 as u64) << 30) | ((i4 as u64) << 39);
    VirtAddr::new(addr)
}

fn virt2idx(addr: VirtAddr) -> (usize, usize, usize, usize) {
    let addr = addr.as_u64();
    let idx4 = (addr >> 39 & 0x1FF) as usize;
    let idx3 = (addr >> 30 & 0x1FF) as usize;
    let idx2 = (addr >> 21 & 0x1FF) as usize;
    let idx1 = (addr >> 12 & 0x1FF) as usize;
    (idx4, idx3, idx2, idx1)
}

fn idx2phys(i4: usize, i3: usize, i2: usize, i1: usize) -> PhysAddr {
    let addr = ((i1 as u64) << 12) | ((i2 as u64) << 21) | ((i3 as u64) << 30) | ((i4 as u64) << 39);
    PhysAddr::new(addr)
}

fn phys2idx(addr: PhysAddr) -> (usize, usize, usize, usize) {
    let addr = addr.as_u64();
    let idx4 = (addr >> 39 & 0x1FF) as usize;
    let idx3 = (addr >> 30 & 0x1FF) as usize;
    let idx2 = (addr >> 21 & 0x1FF) as usize;
    let idx1 = (addr >> 12 & 0x1FF) as usize;
    (idx4, idx3, idx2, idx1)
}

#[repr(align(4096))]
#[repr(C)]
struct PageAllocator<const N: usize> where [u8; (N + 7) / 8]:, [(); N - 1]: {
    buffer: [[u8; 4096]; N],
    allocated: [u8; (N + 7) / 8]
}

impl<const N: usize> PageAllocator<N> where [u8; (N + 7) / 8]:, [(); N - 1]: {
    pub const fn new() -> Self {
        Self {
            buffer: [[0; 4096]; N],
            allocated: [0; (N + 7) / 8]
        }
    }

    pub fn owns(&self, addr: u64) -> bool {
        (self.buffer.as_ptr() as u64 - addr) >> 12 < N as u64
    }
}

static mut PAGE_ALLOCATOR: PageAllocator<564> = PageAllocator::new();

unsafe impl<const N: usize> FrameAllocator<Size4KiB> for PageAllocator<N> where [u8; (N + 7) / 8]:, [(); N - 1]: {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        for i in 0..(N + 7) / 8 - 1 {
            if self.allocated[i] != 0xFF {
                let b = &mut self.allocated[i];
                for j in 0..8 {
                    let bit = 1<<j;
                    if *b & bit == 0 {
                        *b |= bit;
                        let addr = PhysAddr::new(&self.buffer[i * 8 + j] as *const _ as u64);
                        let frame = PhysFrame::from_start_address(addr).unwrap();
                        return Some(frame);
                    }
                }
            } 
        }
        None
    }
}

impl<const N: usize> FrameDeallocator<Size4KiB> for PageAllocator<N> where [u8; (N + 7) / 8]:, [(); N - 1]: {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let addr = frame.start_address().as_u64();
        let base_addr = self.buffer.as_ptr() as u64;
        let offset = addr - base_addr;
        let page_offset = addr >> 12;
        if page_offset as usize >= N {
            panic!("Tried deallocating frame not belonging to this allocator");
        }
        self.allocated[page_offset as usize / 8] &= !(1<<page_offset);
    }
}

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

struct Pager {
    count: usize,
    max: usize
}

impl Pager {
    pub fn new(max: usize) -> Self {
        Self { count: 0, max }
    }

    pub fn next(&mut self, st: &SystemTable<Boot>) {
        self.count += 1;
        if self.count >= self.max {
            wait_for_key(st);
            self.count = 0;
        }
    }
}

fn wait_for_key(st: &SystemTable<Boot>) {
    st.stdin().reset(false).unwrap().unwrap();
    st.boot_services().wait_for_event(&mut [st.stdin().wait_for_key_event()]).unwrap().unwrap();
}

fn is_mapped(pml4: &mut PageTable, addr: VirtAddr) -> bool {
    let (idx4, idx3, idx2, idx1) = virt2idx(addr);
    unsafe {
        if !pml4[idx4].flags().contains(PageTableFlags::PRESENT) {
            return false;
        }
        let page_table_3 = (pml4[idx4].addr().as_u64() as *mut PageTable).as_mut().unwrap();
        if !page_table_3[idx3].flags().contains(PageTableFlags::PRESENT) {
            return false;
        }
        if page_table_3[idx3].flags().contains(PageTableFlags::HUGE_PAGE) {
            return true;
        }
        let page_table_2 = (page_table_3[idx3].addr().as_u64() as *mut PageTable).as_mut().unwrap();
        if !page_table_2[idx2].flags().contains(PageTableFlags::PRESENT) {
            return false;
        }
        if page_table_2[idx2].flags().contains(PageTableFlags::HUGE_PAGE) {
            return true;
        }
        let page_table_1 = (page_table_2[idx2].addr().as_u64() as *mut PageTable).as_mut().unwrap();
        if !page_table_1[idx1].flags().contains(PageTableFlags::PRESENT) {
            return false;
        }
    }
    true
}

fn assert_mapped(pml4: &PageTable, addr: VirtAddr, write_success: bool) {
    let addr = addr.as_u64();
    let idx4 = (addr >> 39 & 0x1FF) as usize;
    let idx3 = (addr >> 30 & 0x1FF) as usize;
    let idx2 = (addr >> 21 & 0x1FF) as usize;
    let idx1 = (addr >> 12 & 0x1FF) as usize;
    unsafe {
        if !pml4[idx4].flags().contains(PageTableFlags::PRESENT) {
            panic!("addr {:x} not mapped in lvl4", addr);
        }
        let page_table_3 = (pml4[idx4].addr().as_u64() as *mut PageTable).as_mut().unwrap();
        if !page_table_3[idx3].flags().contains(PageTableFlags::PRESENT) {
            panic!("addr {:x} not mapped in lvl3", addr);
        }
        if page_table_3[idx3].flags().contains(PageTableFlags::HUGE_PAGE) {
            if write_success {
                println!("addr {:x} is mapped", addr);
            }
            return;
        }
        let page_table_2 = (page_table_3[idx3].addr().as_u64() as *mut PageTable).as_mut().unwrap();
        if !page_table_2[idx2].flags().contains(PageTableFlags::PRESENT) {
            panic!("addr {:x} not mapped in lvl2", addr);
        }
        if page_table_2[idx2].flags().contains(PageTableFlags::HUGE_PAGE) {
            if write_success {
                println!("addr {:x} is mapped", addr);
            }
            return;
        }
        let page_table_1 = (page_table_2[idx2].addr().as_u64() as *mut PageTable).as_mut().unwrap();
        if !page_table_1[idx1].flags().contains(PageTableFlags::PRESENT) {
            panic!("addr {:x} not mapped in lvl1", addr);
        }
        if write_success {
            println!("addr {:x} is mapped", addr);
        }
    }
}

fn wait_debug() {
    #[cfg(feature = "wait_for_gdb")]
    unsafe {
        asm!(
            "mov {tmp}, 1",
            "5: cmp {tmp}, 0",
            "jne 5b",
            tmp = out(reg) _
        );
    }
}
