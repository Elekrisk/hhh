use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    ptr::NonNull,
};

use spin::{Mutex, RwLock};
use x86_64::PhysAddr;
use x86_64::{
    structures::paging::{page_table::PageTableEntry, PageTable, PageTableFlags, PhysFrame},
    VirtAddr,
};

pub fn init(page_table: &'static mut PageTable, allocated_frames: &'static mut [u8]) {
    let mut frame_allocator = FRAME_ALLOCATOR.lock();
    let mut mapper = MAPPER.lock();
    frame_allocator.allocated_frames = allocated_frames;
    mapper.page_table = page_table;
    let start_entry = AllocatedEntry {
        previous: None,
        next: None,
        layout: Layout::from_size_align(0, 1).unwrap(),
    };
    let heap_base = 0xFFFF8100_00000000;
    let frame = frame_allocator.allocate_frame();
    unsafe {
        mapper
            .map(&mut frame_allocator, VirtAddr::new(heap_base), frame)
            .unwrap();
        (heap_base as *mut AllocatedEntry).write(start_entry);
    }
    *ALLOCATOR.base_entry.write() = NonNull::new(heap_base as _).unwrap();
}

static FRAME_ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator {
    allocated_frames: &mut [],
});

pub struct FrameAllocator {
    allocated_frames: &'static mut [u8],
}

impl FrameAllocator {
    pub fn new(allocated_frames: &'static mut [u8]) -> Self {
        Self { allocated_frames }
    }

    pub fn allocate_frame(&mut self) -> PhysFrame {
        for i in 0..self.allocated_frames.len() {
            if self.allocated_frames[i] != 0xFF {
                for j in 0..8 {
                    if self.allocated_frames[i] & 1 << j == 0 {
                        self.allocated_frames[i] |= 1 << j;
                        return PhysFrame::from_start_address(PhysAddr::new(
                            (i * 8 + j) as u64 * 4096,
                        ))
                        .unwrap();
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
        self.allocated_frames[i as usize] &= !(1 << j);
    }
}

static mut TEMP_PAGE_TABLE: PageTable = PageTable::new();
static MAPPER: Mutex<Mapper> = Mutex::new(Mapper {
    page_table: unsafe { &mut TEMP_PAGE_TABLE },
});

pub struct Mapper {
    page_table: &'static mut PageTable,
}

impl Mapper {
    pub fn new(page_table: &'static mut PageTable) -> Self {
        Self { page_table }
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

    pub fn get_physical(&self, virt: VirtAddr) -> Result<PhysFrame, &'static str> {
        if !self.is_mapped(virt) {
            return Err("Address is not mapped");
        }

        let (idx4, idx3, idx2, idx1) = virt2idx(virt);
        let page_table = &self.page_table;
        let page_table = unsafe { page_table[idx4].as_page_table().unwrap() };
        let page_table = unsafe { page_table[idx3].as_page_table().unwrap() };
        let page_table = unsafe { page_table[idx2].as_page_table().unwrap() };
        let frame = page_table[idx1].frame().unwrap();

        Ok(frame)
    }

    pub unsafe fn map(
        &mut self,
        frame_allocator: &mut FrameAllocator,
        virt: VirtAddr,
        frame: PhysFrame,
    ) -> Result<(), &'static str> {
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
        FRAME_ALLOCATOR
            .lock()
            .deallocate_frame(PhysFrame::from_start_address(phys).unwrap());
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
static ALLOCATOR: Allocator = Allocator {
    base_entry: RwLock::new(NonNull::dangling()),
};

unsafe impl Sync for Allocator {}

pub struct Allocator {
    base_entry: RwLock<NonNull<AllocatedEntry>>,
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        println!(
            "alloc request with size {:#x} align {:#x}",
            layout.size(),
            layout.align()
        );
        let mut current = self.base_entry.write();
        let align = layout.align();
        let size = layout.size();
        let mut current = current;

        while let Some(mut next) = (*current).as_ref().next {
            let after_data = current.as_ref().after_data();
            let min_self_ptr = align_up(after_data, AllocatedEntry::ALIGN);
            let data_ptr = align_up(min_self_ptr + AllocatedEntry::SIZE, layout.align());
            let after_data = data_ptr + layout.align();
            let next_start = next.as_ref().self_ptr();
            if next_start >= after_data {
                continue;
            }
            return AllocatedEntry::new_after(*current, Some(next), layout).as_ptr() as _;
        }

        return AllocatedEntry::new_after(*current, None, layout).as_ptr() as _;
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let data_ptr = ptr as usize;
        let entry =
            NonNull::<AllocatedEntry>::new(align_down(data_ptr, AllocatedEntry::ALIGN) as _)
                .unwrap();
        let prev = entry.as_ref().previous;
        let next = entry.as_ref().next;

        match (prev, next) {
            (Some(mut prev), Some(mut next)) => {
                prev.as_mut().next = Some(next);
                next.as_mut().previous = Some(prev);
            }
            (Some(mut prev), _) => prev.as_mut().next = None,
            (_, Some(mut next)) => next.as_mut().previous = None,
            _ => unreachable!(),
        }

        let first_page = if let Some(prev) = prev {
            (prev.as_ref().last_page() + 1).max(entry.as_ref().first_page())
        } else {
            entry.as_ref().first_page()
        };
        let last_page = if let Some(next) = next {
            (next.as_ref().first_page() - 1).min(entry.as_ref().last_page())
        } else {
            entry.as_ref().last_page()
        };

        let mut mapper = MAPPER.lock();
        for page in first_page..=last_page {
            let virt = VirtAddr::new((page as u64) << 12);
            mapper.unmap(virt).unwrap();
        }
    }
}

/// This should be placed immediately in front of the allocated data
pub struct AllocatedEntry {
    previous: Option<NonNull<AllocatedEntry>>,
    next: Option<NonNull<AllocatedEntry>>,
    layout: Layout,
}

impl AllocatedEntry {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ALIGN: usize = core::mem::align_of::<Self>();

    unsafe fn new_after(
        mut previous: NonNull<AllocatedEntry>,
        next: Option<NonNull<AllocatedEntry>>,
        layout: Layout,
    ) -> NonNull<AllocatedEntry> {
        let after_previous = previous.as_ref().after_data();
        let min_self_ptr = align_up(after_previous, Self::ALIGN);
        let data_ptr = align_up(min_self_ptr + Self::SIZE, layout.align());
        let self_ptr = align_down(data_ptr - Self::SIZE, Self::ALIGN);

        let entry = Self {
            previous: Some(previous),
            next: next,
            layout,
        };

        let mut frame_allocator = FRAME_ALLOCATOR.lock();
        let mut mapper = MAPPER.lock();

        let first_page = self_ptr >> 12;
        let last_page = (data_ptr + layout.size()) >> 12;

        for page in first_page..=last_page {
            let virt = VirtAddr::new((page as u64) << 12);
            if !mapper.is_mapped(virt) {
                let frame = frame_allocator.allocate_frame();
                mapper.map(&mut frame_allocator, virt, frame).unwrap();
            }
        }

        let self_ptr = NonNull::new(self_ptr as *mut Self).unwrap();
        self_ptr.as_ptr().write(entry);
        previous.as_mut().next = Some(self_ptr);
        next.map(|mut next| next.as_mut().previous = Some(self_ptr));
        self_ptr
    }

    fn first_page(&self) -> usize {
        self as *const _ as usize >> 12
    }

    fn last_page(&self) -> usize {
        self.after_data() >> 12
    }

    fn self_ptr(&self) -> usize {
        self as *const _ as usize
    }

    fn data_ptr(&self) -> usize {
        align_up(self.self_ptr() + Self::SIZE, self.layout.align())
    }

    fn after_data(&self) -> usize {
        self.data_ptr() + self.layout.size()
    }
}

fn align_down(addr: usize, align: usize) -> usize {
    let ret = addr - addr % align;
    ret
}

fn align_up(addr: usize, align: usize) -> usize {
    let m = addr % align;
    let ret = if m == 0 { addr } else { addr - m + align };
    ret
}

fn idx2virt(i4: usize, i3: usize, i2: usize, i1: usize) -> VirtAddr {
    let addr =
        ((i1 as u64) << 12) | ((i2 as u64) << 21) | ((i3 as u64) << 30) | ((i4 as u64) << 39);
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
    let addr =
        ((i1 as u64) << 12) | ((i2 as u64) << 21) | ((i3 as u64) << 30) | ((i4 as u64) << 39);
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
        if !self.is_unused()
            && self.flags().contains(PageTableFlags::PRESENT)
            && !self.flags().contains(PageTableFlags::HUGE_PAGE)
        {
            Some(
                ((self.addr().as_u64() | 0xFFFFFF80_00000000) as *const PageTable)
                    .as_ref()
                    .unwrap(),
            )
        } else {
            None
        }
    }

    unsafe fn as_page_table_mut(&mut self) -> Option<&mut PageTable> {
        if !self.is_unused()
            && self.flags().contains(PageTableFlags::PRESENT)
            && !self.flags().contains(PageTableFlags::HUGE_PAGE)
        {
            Some(
                ((self.addr().as_u64() | 0xFFFFFF80_00000000) as *mut PageTable)
                    .as_mut()
                    .unwrap(),
            )
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
        mapper
            .map(
                &mut frame_allocator,
                virt,
                PhysFrame::containing_address(phys),
            )
            .unwrap()
    };
}
