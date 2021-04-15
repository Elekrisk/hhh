use core::{alloc::GlobalAlloc, cell::UnsafeCell};

use spin::Mutex;
use x86_64::{VirtAddr, structures::paging::{PageTable, PageTableFlags, PhysFrame, page_table::PageTableEntry}};
use x86_64::PhysAddr;

pub fn init(page_table: &'static mut PageTable, allocated_frames: &'static mut [u8]) {
    FRAME_ALLOCATOR.lock().allocated_frames = allocated_frames;
    MAPPER.lock().page_table = page_table;
}

static FRAME_ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator { allocated_frames: &mut [] });

pub struct FrameAllocator {
    allocated_frames: &'static mut [u8]
}

impl FrameAllocator {
    pub fn new( allocated_frames: &'static mut [u8]) -> Self {
        Self {
            allocated_frames
        }
    }

    pub fn allocate_frame(&mut self) -> PhysFrame {
        for i in 0..self.allocated_frames.len() {
            if self.allocated_frames[i] != 0xFF {
                for j in 0..8 {
                    if self.allocated_frames[i] & 1<<j == 0 {
                        self.allocated_frames[i] |= 1<<j;
                        return PhysFrame::from_start_address(PhysAddr::new((i * 8 + j) as u64 * 4096)).unwrap();
                    }
                }
            }
        }
        panic!("No physical frames left to allocate");
    }

    pub fn deallocate_frame(&mut self, frame: PhysFrame) {
        let addr = frame.start_address().as_u64();
        let frame_nr = addr >> 12;
        let i = frame_nr % 8;
        let j = frame_nr / 8;
        self.allocated_frames[i as usize] &= !(1<<j);
    }
}

static mut TEMP_PAGE_TABLE: PageTable = PageTable::new();
static MAPPER: Mutex<Mapper> = Mutex::new(Mapper { page_table: unsafe { &mut TEMP_PAGE_TABLE } });

pub struct Mapper {
    page_table: &'static mut PageTable
}

impl Mapper {
    pub fn new(page_table: &'static mut PageTable) -> Self {
        Self {
            page_table
        }
    }

    pub fn is_mapped(&self, virt: VirtAddr) -> bool {
        let (idx4, idx3, idx2, idx1) = virt2idx(virt);

        if self.page_table[idx4].is_unused() {
            return false;
        }
        let pdp = unsafe { self.page_table[idx4].as_page_table() }.unwrap();

        if pdp[idx3].is_unused() {
            return false;
        } else if pdp[idx3].flags().contains(PageTableFlags::HUGE_PAGE) {
            return true;
        }
        let pd = unsafe { pdp[idx3].as_page_table() }.unwrap();
        
        if pd[idx2].is_unused() {
            return false;
        } else if pd[idx2].flags().contains(PageTableFlags::HUGE_PAGE) {
            return true;
        }
        let pt = unsafe { pd[idx2].as_page_table() }.unwrap();

        if pt[idx1].is_unused() {
            return false;
        }

        true
    }

    pub unsafe fn map(&mut self, frame_allocator: &mut FrameAllocator, virt: VirtAddr, frame: PhysFrame) -> Result<(), &'static str> {
        let (idx4, idx3, idx2, idx1) = virt2idx(virt);
        
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        if self.page_table[idx4].is_unused() {
            let pdp_addr = frame_allocator.allocate_frame().start_address();
            self.page_table[idx4].set_addr(pdp_addr, flags);
        }
        let pdp = self.page_table[idx4].as_page_table_mut().unwrap();

        if pdp[idx3].is_unused() {
            let pd_addr = frame_allocator.allocate_frame().start_address();
            pdp[idx3].set_addr(pd_addr, flags);
        } else if pdp[idx3].flags().contains(PageTableFlags::HUGE_PAGE) {
            return Err("Trying to map to pdp entry containing a 1G page");
        }
        let pd = pdp[idx3].as_page_table_mut().unwrap();
        
        if pd[idx2].is_unused() {
            let pt_addr = frame_allocator.allocate_frame().start_address();
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

    pub unsafe fn unmap(&mut self, virt: VirtAddr) -> Result<(), &'static str> {
        let (idx4, idx3, idx2, idx1) = virt2idx(virt);

        if self.page_table[idx4].is_unused() {
            return Err("Tried unmapping unmapped page (level 4)");
        }
        let pdp = self.page_table[idx4].as_page_table_mut().unwrap();
        
        if pdp[idx3].is_unused() {
            return Err("Tried unmapping unmapped page (level 3)");
        } else if pdp[idx3].flags().contains(PageTableFlags::HUGE_PAGE) {
            return Err("Tried unmapping huge page (1G)");
        }
        let pd = pdp[idx3].as_page_table_mut().unwrap();
        
        if pd[idx2].is_unused() {
            return Err("Tried unmapping unmapped page (level 2)");
        } else if pd[idx2].flags().contains(PageTableFlags::HUGE_PAGE) {
            return Err("Tried unmapping huge page (2M)");
        }
        let pt = pd[idx2].as_page_table_mut().unwrap();

        if pt[idx1].is_unused() {
            return Err("Tried unmapping unmapped page (level 1)");
        }

        // Start unmapping process

        // Unmap frame
        let phys = idx2phys(idx4, idx3, idx2, idx1);
        FRAME_ALLOCATOR.lock().deallocate_frame(PhysFrame::from_start_address(phys).unwrap());
        // Unmap page
        pt[idx1].set_unused();

        // Due to Rust's aliasing rules, we add the addresses of all page tables to be deallocated to an array to deallocate them after we
        // have unmapped them from the PML4.
        let mut to_deallocate = [None; 3]; // Max number of page_tables to deallocate is 3; a PT, a PD and a PDP

        // Check if all entries in this PT is unused, and if so, set the corresponding PDE to unused and add the address
        // of the PT to be deallocated.
        if pt.iter().all(|e| e.is_unused()) {
            let pt_addr = VirtAddr::new(pt as *mut _ as u64);
            to_deallocate[0] = Some(pt_addr);
            pd[idx2].set_unused();
        }

        // Do the same for the PD
        if pd.iter().all(|e| e.is_unused()) {
            let pd_addr = VirtAddr::new(pd as *mut _ as u64);
            to_deallocate[1] = Some(pd_addr);
            pdp[idx3].set_unused();
        }

        // Same for PDP
        if pdp.iter().all(|e| e.is_unused()) {
            let pdp_addr = VirtAddr::new(pdp as *mut _ as u64);
            to_deallocate[2] = Some(pdp_addr);
            self.page_table[idx4].set_unused();
        }

        // Deallocate the pages which held the page tables which are now unused;
        for addr in to_deallocate.iter().filter_map(|v| *v) {
            self.unmap(addr)?;
        }

        Ok(())
    }
}

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

pub struct Allocator;

impl Allocator {
    const BASE_ADDRESS: u64 = 0x00008000_00000000;
}

unsafe impl GlobalAlloc for Allocator {

    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        assert!(layout.align() <= 4096);
        assert!(layout.size() <= 4096);

        let mut mapper = MAPPER.lock();

        let mut address = None;
        for page in 0.. {
            let addr = VirtAddr::new(Self::BASE_ADDRESS + (page << 12));
            if mapper.is_mapped(addr) {
                continue;
            } else {
                address = Some(addr);
                break;
            }
        }
        let addr = address.unwrap();
        let mut frame_allocator = FRAME_ALLOCATOR.lock();
        let frame = frame_allocator.allocate_frame();
        mapper.map(&mut frame_allocator, addr, frame).unwrap();
        addr.as_mut_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        todo!()
    }
}


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

trait AsPageTable {
    unsafe fn as_page_table(&self) -> Option<&PageTable>;
    unsafe fn as_page_table_mut(&mut self) -> Option<&mut PageTable>;
}

impl AsPageTable for PageTableEntry {
    unsafe fn as_page_table(&self) -> Option<&PageTable> {
        if !self.is_unused() && self.flags().contains(PageTableFlags::PRESENT) && !self.flags().contains(PageTableFlags::HUGE_PAGE) {
            Some(((self.addr().as_u64() | 0xFFFFFF80_00000000) as *const PageTable).as_ref().unwrap())
        } else {
            None
        }
    }

    unsafe fn as_page_table_mut(&mut self) -> Option<&mut PageTable> {
        if !self.is_unused() && self.flags().contains(PageTableFlags::PRESENT) && !self.flags().contains(PageTableFlags::HUGE_PAGE) {
            Some(((self.addr().as_u64() | 0xFFFFFF80_00000000) as *mut PageTable).as_mut().unwrap())
        } else {
            None
        }
    }
}

pub fn map_phys_offset(virt: VirtAddr) {
    let phys = PhysAddr::new(virt.as_u64() & 0x0000007F_FFFFFFFF);
    let mut mapper = MAPPER.lock();
    if mapper.is_mapped(virt) {
        return;
    }
    let mut frame_allocator = FRAME_ALLOCATOR.lock();
    unsafe {
        mapper.map(&mut frame_allocator, virt, PhysFrame::containing_address(phys)).unwrap()
    };
}
