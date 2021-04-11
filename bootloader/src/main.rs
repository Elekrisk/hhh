#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(abi_efiapi)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(asm)]
#![feature(vec_into_raw_parts)]
#![feature(abi_x86_interrupt)]
#![feature(assoc_char_funcs)]

extern crate rlibc;
extern crate uefi;
extern crate uefi_services;

mod elf;
mod writer;

use alloc::vec::Vec;
use common::Framebuffer;
use elf::{Elf, EntryType, HeaderEntry, SectionType};
use uefi::{prelude::*, proto::{console::gop::GraphicsOutput, media::{file::{File, FileAttribute, FileMode, FileType}, fs::SimpleFileSystem}}, table::boot::{AllocateType, MemoryDescriptor, MemoryType}};
use uefi::{Handle, Status, table::{Boot, SystemTable}};
use log::info;
use x86_64::{PhysAddr, VirtAddr, instructions, structures::{gdt::{Descriptor, GlobalDescriptorTable}, idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode}, paging::{FrameAllocator, OffsetPageTable, PageTable, PageTableFlags, Size4KiB, frame::{self, PhysFrame}, page_table::PageTableEntry}}};
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
        
        let mut lowest_base = u64::MAX;
        for segment in &kernel_elf.program_headers {
            if segment.entry_type == EntryType::Load {
                lowest_base = lowest_base.min(segment.virtual_addr);
            }
        }

        let mut to_allocate = Vec::new();

        for segment in &kernel_elf.program_headers {
            if segment.entry_type == EntryType::Load {
                let start = segment.virtual_addr - lowest_base;
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
            }
        }

        let mut has_changed = true;
        while has_changed {
            has_changed = false;
            let mut i = 0;
            while i < to_allocate.len() {
                let mut j = i + 1;
                while j < to_allocate.len() {
                    let (a1, b1) = to_allocate[i];
                    let (a2, b2) = to_allocate[j];

                    if a1 <= a2 && b1 >= a2 || a1 <= b2 && b1 >= b2 || a1 >= a2 && b1 <= b2 || b1 + 1 == a2 || b2 + 1 == a1 {
                        let first = a1.min(a2);
                        let last = b1.max(b2);
                        to_allocate.remove(j);
                        to_allocate.remove(i);
                        to_allocate.push((first, last));
                        has_changed = true;
                    }
                    j += 1;
                }
                i += 1;
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
                let actual_start = start + offset - lowest_base;
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
        info!("Entry is at {:x}", kernel_elf.entry + offset - lowest_base);

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
        unsafe {
            FRAMEBUFFER = framebuffer.ptr;
        }
        
        
        let actual_entry = offset + kernel_elf.entry - lowest_base;
        
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

        info!("Press any key to view memmap");
        wait_for_key(&system_table);
        
        let memory_map_size = system_table.boot_services().memory_map_size();
        let mut memory_map_buffer = Vec::new();
        memory_map_buffer.resize(memory_map_size + 256, 0);
        let mut pager = Pager::new(24);
        for entry in system_table.boot_services().memory_map(&mut memory_map_buffer).unwrap().unwrap().1 {
            info!("{:?} phys {:x} virt {:x} page_count {}", entry.ty, entry.phys_start, entry.virt_start, entry.page_count);
            pager.next(&system_table);
        }
        
        info!("Press any key to continue");
        wait_for_key(&system_table);
        // #[cfg(feature = "wait_for_gdb")]
        // info!("Will wait for GDB after jump");
        // info!("Press any key to jump to kernel");
        // wait_for_key(&system_table);

        // info!("Jump!");
        
        (unsafe { core::mem::transmute::<_, extern "sysv64" fn(Framebuffer)>(actual_entry) }, framebuffer)
    };
        
    writer::init(framebuffer.clone());
    writer::clear();
    let mut rows_written = 0;
    
    let memmapsize = system_table.boot_services().memory_map_size();
    let desc_size = core::mem::size_of::<MemoryDescriptor>();
    let vec_size = memmapsize + desc_size;
    let mut memmapbuffer = Vec::with_capacity(vec_size);
    memmapbuffer.resize(memmapsize + 128, 0);
    let (st, memmap) = system_table.exit_boot_services(image_handle, &mut memmapbuffer).unwrap_success();
    
    unsafe {
        let code_segment = GDT.add_entry(Descriptor::kernel_code_segment());
        let data_segment = GDT.add_entry(Descriptor::kernel_data_segment());
        GDT.load();
        instructions::segmentation::load_ss(data_segment);
        instructions::segmentation::set_cs(code_segment);

        IDT.page_fault.set_handler_fn(page_fault);
        IDT.double_fault.set_handler_fn(double_fault);

        IDT.load();

        for (i, entry) in PAGE_TABLE_3.iter_mut().enumerate() {
            let addr = PhysAddr::new(i as u64 * 1024 * 1024 * 1024);
            let flags = PageTableFlags::GLOBAL | PageTableFlags::HUGE_PAGE | PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            entry.set_addr(addr, flags);
        }
        let addr = PhysAddr::new(&PAGE_TABLE_3 as *const PageTable as u64);
        let flags = PageTableFlags::GLOBAL | PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        PAGE_TABLE_4[511].set_addr(addr, flags);
        let mut page_table = OffsetPageTable::new(&mut PAGE_TABLE_4, index2virt(511, 0, 0, 0));

        for memmap_entry in memmap {
            match memmap_entry.ty {
                MemoryType::LOADER_CODE |
                MemoryType::LOADER_DATA |
                MemoryType::BOOT_SERVICES_DATA => {
                    let start = memmap_entry.phys_start;
                    let page_count = memmap_entry.page_count;

                    if memmap_entry.ty == MemoryType::LOADER_DATA {
                        writer::write_str("Mapping LOADER_DATA memory at ");
                        writer::write_hex(start);
                        writer::write_str("..");
                        writer::write_hex(start + page_count * 4096);
                        writer::write_str("\n");
                        rows_written += 1;
                    }
                    
                    for i in 0..page_count {
                        // use x86_64::structures::paging::Mapper;
                        let start = start + i * 4096;
                        let frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(start));
                        // match page_table.identity_map(frame, PageTableFlags::GLOBAL | PageTableFlags::PRESENT | PageTableFlags::WRITABLE, &mut PAGE_ALLOCATOR) {
                        //     Ok(v) => v.flush(),
                        //     Err(e) => {
                        //         for i in 0..200 {
                        //             (FRAMEBUFFER as *mut u32).add(i).write_volatile(0x00ff00);
                        //         }
                        //     }
                        // }

                        // writer::write_str("addr: ");
                        // writer::write_hex(frame.start_address().as_u64());

                        let page_table = page_table.level_4_table();
                        let idx4 = (start >> 39 & 0x1FF) as usize;
                        let idx3 = (start >> 30 & 0x1FF) as usize;
                        let idx2 = (start >> 21 & 0x1FF) as usize;
                        let idx1 = (start >> 12 & 0x1FF) as usize;

                        // writer::write_str(" ");
                        // writer::write_u64(idx4 as _);
                        // writer::write_str(" ");
                        // writer::write_u64(idx3 as _);
                        // writer::write_str(" ");
                        // writer::write_u64(idx2 as _);
                        // writer::write_str(" ");
                        // writer::write_u64(idx1 as _);
                        // writer::write_str("\n");
                        
                        let flags = PageTableFlags::GLOBAL | PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
                        if page_table[idx4].is_unused() {
                            let page_table_3 = PAGE_ALLOCATOR.allocate_frame().unwrap2();
                            (page_table_3.start_address().as_u64() as *mut PageTable).write(PageTable::new());
                            page_table[idx4].set_addr(page_table_3.start_address(), flags);
                        }
                        let page_table = (page_table[idx4].addr().as_u64() as *mut PageTable).as_mut().unwrap();
                        if page_table[idx3].is_unused() {
                            let page_table_2 = PAGE_ALLOCATOR.allocate_frame().unwrap2();
                            (page_table_2.start_address().as_u64() as *mut PageTable).write(PageTable::new());
                            page_table[idx3].set_addr(page_table_2.start_address(), flags);
                        }
                        let page_table = (page_table[idx3].addr().as_u64() as *mut PageTable).as_mut().unwrap();
                        if page_table[idx2].is_unused() {
                            let page_table_1 = PAGE_ALLOCATOR.allocate_frame().unwrap2();
                            (page_table_1.start_address().as_u64() as *mut PageTable).write(PageTable::new());
                            page_table[idx2].set_addr(page_table_1.start_address(), flags);
                        }
                        let page_table = (page_table[idx2].addr().as_u64() as *mut PageTable).as_mut().unwrap();
                        if page_table[idx1].is_unused() {
                            page_table[idx1].set_frame(frame, flags);
                        } else {
                            writer::write_str("Overlapping pages: ");
                            writer::write_hex(start);
                            loop {}
                        }
                    }
                },
                _ => {}
            }
        }
        FRAMEBUFFER = (FRAMEBUFFER as u64 | index2virt(511, 0, 0, 0).as_u64()) as *mut u8;
        x86_64::registers::control::Cr3::write(PhysFrame::from_start_address(PhysAddr::new((&PAGE_TABLE_4) as *const PageTable as u64)).unwrap(), x86_64::registers::control::Cr3Flags::empty());
    }
    let framebuffer = Framebuffer { ptr: (framebuffer.ptr as u64 | 0xFFFFFF8000000000) as *mut u8, ..framebuffer };
    writer::init(framebuffer.clone());
    for _ in 0..rows_written {
        writer::write_char('\n');
    }

    // let entry_addr = entry as u64;
    // let idx4 = (entry_addr >> 39 & 0x1FF) as usize;
    // let idx3 = (entry_addr >> 30 & 0x1FF) as usize;
    // let idx2 = (entry_addr >> 21 & 0x1FF) as usize;
    // let idx1 = (entry_addr >> 12 & 0x1FF) as usize;

    // unsafe {
    //     if !PAGE_TABLE_4[idx4].flags().contains(PageTableFlags::PRESENT) {
    //         writer::write_str("entry ");
    //         writer::write_hex(entry_addr);
    //         writer::write_str(" not mapped in lvl4");
    //         loop {}
    //     }
    //     let page_table_3 = (PAGE_TABLE_4[idx4].addr().as_u64() as *mut PageTable).as_mut().unwrap();
    //     if !page_table_3[idx3].flags().contains(PageTableFlags::PRESENT) {
    //         writer::write_str("entry ");
    //         writer::write_hex(entry_addr);
    //         writer::write_str(" not mapped in lvl3");
    //         loop {}
    //     }
    //     let page_table_2 = (page_table_3[idx3].addr().as_u64() as *mut PageTable).as_mut().unwrap();
    //     if !page_table_2[idx2].flags().contains(PageTableFlags::PRESENT) {
    //         writer::write_str("entry ");
    //         writer::write_hex(entry_addr);
    //         writer::write_str(" not mapped in lvl2");
    //         loop {}
    //     }
    //     let page_table_1 = (page_table_2[idx2].addr().as_u64() as *mut PageTable).as_mut().unwrap();
    //     if !page_table_1[idx1].flags().contains(PageTableFlags::PRESENT) {
    //         writer::write_str("entry ");
    //         writer::write_hex(entry_addr);
    //         writer::write_str(" not mapped in lvl1");
    //         loop {}
    //     }
    //     writer::write_str("entry is mapped");
    //     loop {}
    // }

    fn index2virt(i4: u16, i3: u16, i2: u16, i1: u16) -> VirtAddr {
        let addr = ((i1 as u64) << 12) | ((i2 as u64) << 21) | ((i3 as u64) << 30) | ((i4 as u64) << 39);
        VirtAddr::new(addr)
    }

    wait_debug();

    entry(framebuffer.clone());

    writer::write_str("DONE");

    loop {}
}

trait Unwrap2<T> {
    fn unwrap2(self) -> T;
}

impl<T, E> Unwrap2<T> for Result<T, E> {
    fn unwrap2(self) -> T {
        match self {
            Ok(v) => v,
            Err(_) => {
                unsafe {
                    for i in 100..200 {
                        (FRAMEBUFFER as *mut u32).add(i).write(0xFF0000);
                    }
                }

                loop {}
            }
        }
    }
}

impl<T> Unwrap2<T> for Option<T> {
    fn unwrap2(self) -> T {
        match self {
            Some(v) => v,
            None => {
                unsafe {
                    for i in 100..200 {
                        (FRAMEBUFFER as *mut u32).add(i).write(0xFF0000);
                    }
                }

                loop {}
            }
        }
    }
}

#[repr(align(4096))]
#[repr(C)]
struct PageAllocator {
    buffer: [[u8; 4096]; PageAllocator::BUFFER_SIZE],
    count: usize
}

impl PageAllocator {
    const BUFFER_SIZE: usize = 32;

    pub const fn new() -> Self {
        Self {
            buffer: [[0; 4096]; PageAllocator::BUFFER_SIZE],
            count: 0
        }
    }
}

static mut PAGE_ALLOCATOR: PageAllocator = PageAllocator::new();

unsafe impl FrameAllocator<Size4KiB> for PageAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        if self.count < Self::BUFFER_SIZE {
            let addr = PhysAddr::new((&self.buffer[self.count]) as *const [u8; 4096] as u64);
            self.count += 1;
            Some(PhysFrame::from_start_address(addr).unwrap())
        } else {
            unsafe {
                for i in 0..100 {
                    (FRAMEBUFFER as *mut u32).add(i).write_volatile(0x3333FF);
                }
            }
            None
        }
    }
}

static mut FRAMEBUFFER: *mut u8 = 0 as *mut u8;

pub extern "x86-interrupt" fn page_fault(stack_frame: &mut InterruptStackFrame, error_code: PageFaultErrorCode) {
    writer::write_str("\npage fault");
    loop {}
}

pub extern "x86-interrupt" fn double_fault(stack_frame: &mut InterruptStackFrame, error_code: u64) -> ! {
    writer::write_str("\ndouble fault");
    loop {}
}

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

static mut PAGE_TABLE_4: PageTable = PageTable::new();
static mut PAGE_TABLE_3: PageTable = PageTable::new();

const NEW_PAGE_TABLE: PageTable = PageTable::new();
const PAGE_TABLE_MAX_COUNT: usize = 32;
static mut PAGE_TABLE_ARRAY: [PageTable; PAGE_TABLE_MAX_COUNT] = [NEW_PAGE_TABLE; PAGE_TABLE_MAX_COUNT];
static mut PAGE_TABLE_COUNT: usize = 0;

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
