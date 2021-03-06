use core::iter::Chain;
use core::ops::{Deref, DerefMut};
use core::slice::Iter;

use alloc::boxed::Box;
use alloc::fmt;
use core::alloc::{GlobalAlloc, Layout};

use crate::allocator;
use crate::param::*;
use crate::vm::{PhysicalAddr, VirtualAddr};
use crate::ALLOCATOR;

use aarch64::vmsa::*;
use shim::const_assert_size;

#[repr(C)]
pub struct Page([u8; PAGE_SIZE]);
const_assert_size!(Page, PAGE_SIZE);

impl Page {
    pub const SIZE: usize = PAGE_SIZE;
    pub const ALIGN: usize = PAGE_SIZE;

    fn layout() -> Layout {
        unsafe { Layout::from_size_align_unchecked(Self::SIZE, Self::ALIGN) }
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct L2PageTable {
    pub entries: [RawL2Entry; 8192],
}
const_assert_size!(L2PageTable, PAGE_SIZE);

impl L2PageTable {
    /// Returns a new `L2PageTable`
    fn new() -> L2PageTable {
        L2PageTable {
            entries: [RawL2Entry::new(0); 8192]
        }
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
        PhysicalAddr::from(self as *const L2PageTable as u64)
    }
}

#[derive(Copy, Clone)]
pub struct L3Entry(RawL3Entry);

impl L3Entry {
    /// Returns a new `L3Entry`.
    fn new() -> L3Entry {
       L3Entry(RawL3Entry::new(0))
    }

    /// Returns `true` if the L3Entry is valid and `false` otherwise.
    fn is_valid(&self) -> bool {
        self.0.get_value(RawL3Entry::VALID) == EntryValid::Valid 
    }

    /// Extracts `ADDR` field of the L3Entry and returns as a `PhysicalAddr`
    /// if valid. Otherwise, return `None`.
    fn get_page_addr(&self) -> Option<PhysicalAddr> {
        match self.is_valid() {
            true => {
                let paddr: PhysicalAddr = PhysicalAddr::from((self.0.get_value(RawL3Entry::ADDR))<< 16);
                return Some(paddr);
            },
            false => None
        }
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct L3PageTable {
    pub entries: [L3Entry; 8192],
}
const_assert_size!(L3PageTable, PAGE_SIZE);

impl L3PageTable {
    /// Returns a new `L3PageTable`.
    fn new() -> L3PageTable {
        L3PageTable {
            entries: [L3Entry::new(); 8192]
        }
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
        PhysicalAddr::from(self as *const L3PageTable as u64)
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct PageTable {
    pub l2: L2PageTable,
    pub l3: [L3PageTable; 3],
}

impl PageTable {
    /// Returns a new `Box` containing `PageTable`.
    /// Entries in L2PageTable should be initialized properly before return.
    fn new(perm: u64) -> Box<PageTable> {
        let mut page_table = Box::new(PageTable {
            l2: L2PageTable::new(),
            l3: [L3PageTable::new(), L3PageTable::new(), L3PageTable::new()]
        });
        page_table.l2.entries[0].set_value(EntryValid::Valid, RawL2Entry::VALID);
        page_table.l2.entries[0].set_value(EntryType::Table, RawL2Entry::TYPE);
        page_table.l2.entries[0].set_value(EntryAttr::Mem, RawL2Entry::ATTR);
        page_table.l2.entries[0].set_value(perm, RawL2Entry::AP);
        page_table.l2.entries[0].set_value(EntrySh::ISh, RawL2Entry::SH);
        page_table.l2.entries[0].set_value(1, RawL2Entry::AF);
        //Even address for L3 table needs to be right shifted by 16 bits before storing in ADDR
        //field of RawL2Entry
        page_table.l2.entries[0].set_masked(page_table.l3[0].as_ptr().as_u64(), RawL2Entry::ADDR);

        page_table.l2.entries[1].set_value(EntryValid::Valid, RawL2Entry::VALID);
        page_table.l2.entries[1].set_value(EntryType::Table, RawL2Entry::TYPE);
        page_table.l2.entries[1].set_value(EntryAttr::Mem, RawL2Entry::ATTR);
        page_table.l2.entries[1].set_value(perm, RawL2Entry::AP);
        page_table.l2.entries[1].set_value(EntrySh::ISh, RawL2Entry::SH);
        page_table.l2.entries[1].set_value(1, RawL2Entry::AF);
        //Even address for L3 table needs to be right shifted by 16 bits before storing in ADDR
        //field of RawL2Entry
        page_table.l2.entries[1].set_masked(page_table.l3[1].as_ptr().as_u64(), RawL2Entry::ADDR);

        page_table.l2.entries[2].set_value(EntryValid::Valid, RawL2Entry::VALID);
        page_table.l2.entries[2].set_value(EntryType::Table, RawL2Entry::TYPE);
        page_table.l2.entries[2].set_value(EntryAttr::Mem, RawL2Entry::ATTR);
        page_table.l2.entries[2].set_value(perm, RawL2Entry::AP);
        page_table.l2.entries[2].set_value(EntrySh::ISh, RawL2Entry::SH);
        page_table.l2.entries[2].set_value(1, RawL2Entry::AF);
        //Even address for L3 table needs to be right shifted by 16 bits before storing in ADDR
        //field of RawL2Entry
        page_table.l2.entries[2].set_masked(page_table.l3[2].as_ptr().as_u64(), RawL2Entry::ADDR);

        page_table
    }

    /// Returns the (L2index, L3index) extracted from the given virtual address.
    /// L2index should be smaller than the number of L3PageTable.
    ///
    /// # Panics
    ///
    /// Panics if the virtual address is not properly aligned to page size.
    /// Panics if extracted L2index exceeds the number of L3PageTable.
    fn locate(va: VirtualAddr) -> (usize, usize) {
        let va_u64 = va.as_usize();
        if va_u64%PAGE_SIZE != 0 {
            panic!("VirtualAddr {:?} not aligned to page size {:?}", va, PAGE_SIZE);
        }
        let l2_mask = 0x3ffe0000000;
        let l3_mask = 0x0001fff0000;
        let l2_index = (va_u64 & l2_mask)>>29;
        let l3_index = (va_u64 & l3_mask)>>16;
        if l2_index > 2{
            panic!("L2 index {:?} for Virtual Address {:?} is greater than 1", l2_index, va);
        }
        (l2_index, l3_index)
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is valid.
    /// Otherwise, `false` is returned.
    pub fn is_valid(&self, va: VirtualAddr) -> bool {
        let (l2_index, l3_index) = PageTable::locate(va);
        self.l3[l2_index].entries[l3_index].0.get_value(RawL3Entry::VALID) == EntryValid::Valid
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is invalid.
    /// Otherwise, `true` is returned.
    pub fn is_invalid(&self, va: VirtualAddr) -> bool {
        !self.is_valid(va)
    }

    /// Set the given RawL3Entry `entry` to the L3Entry indicated by the given virtual
    /// address.
    pub fn set_entry(&mut self, va: VirtualAddr, entry: RawL3Entry) -> &mut Self {
        let (l2_index, l3_index) = PageTable::locate(va);
        self.l3[l2_index].entries[l3_index].0 = entry;
        self
    }

    /// Returns a base address of the pagetable. The returned `PhysicalAddr` value
    /// will point the start address of the L2PageTable.
    pub fn get_baddr(&self) -> PhysicalAddr {
        self.l2.as_ptr()
    }
}

// Implement `IntoIterator` for `&PageTable`.
impl<'a> IntoIterator for &'a mut PageTable {
    type Item = &'a L3Entry;
    type IntoIter = Chain<Iter<'a, L3Entry>, Iter<'a, L3Entry> >;
    fn into_iter(self) -> Self::IntoIter {
        self.l3[0].entries.iter().chain(self.l3[1].entries.iter())
    }
}

pub struct KernPageTable(Box<PageTable>);

impl KernPageTable {
    /// Returns a new `KernPageTable`. `KernPageTable` should have a `Pagetable`
    /// created with `KERN_RW` permission.
    ///
    /// Set L3entry of ARM physical address starting at 0x00000000 for RAM and
    /// physical address range from `IO_BASE` to `IO_BASE_END` for peripherals.
    /// Each L3 entry should have correct value for lower attributes[10:0] as well
    /// as address[47:16]. Refer to the definition of `RawL3Entry` in `vmsa.rs` for
    /// more details.
    pub fn new() -> KernPageTable {
        let mut page_table = PageTable::new(aarch64::EntryPerm::KERN_RW);
        let (_, end) = allocator::memory_map().unwrap();
        let mut addr = 0;
        while addr + PAGE_SIZE <= end {
            let mut l3_entry = RawL3Entry::new(0);
            let saddr = (addr as u64) >> 16;
            l3_entry.set_value(EntryValid::Valid, RawL3Entry::VALID);
            l3_entry.set_value(PageType::Page, RawL3Entry::TYPE);
            l3_entry.set_value(EntryAttr::Mem, RawL3Entry::ATTR);
            l3_entry.set_value(EntryPerm::KERN_RW, RawL3Entry::AP);
            l3_entry.set_value(EntrySh::ISh, RawL3Entry::SH);
            l3_entry.set_value(1, RawL3Entry::AF);
            l3_entry.set_value(saddr, RawL3Entry::ADDR);
            page_table.set_entry(VirtualAddr::from(addr), l3_entry);
            addr+= PAGE_SIZE;
        }
        
        addr = IO_BASE;
        while addr + PAGE_SIZE <= IO_BASE_END {
            let mut l3_entry = RawL3Entry::new(0);
            let saddr = (addr as u64) >> 16;
            l3_entry.set_value(EntryValid::Valid, RawL3Entry::VALID);
            l3_entry.set_value(PageType::Page, RawL3Entry::TYPE);
            l3_entry.set_value(EntryAttr::Dev, RawL3Entry::ATTR);
            l3_entry.set_value(EntryPerm::KERN_RW, RawL3Entry::AP);
            l3_entry.set_value(EntrySh::OSh, RawL3Entry::SH);
            l3_entry.set_value(1, RawL3Entry::AF);
            l3_entry.set_value(saddr, RawL3Entry::ADDR);
            page_table.set_entry(VirtualAddr::from(addr), l3_entry);
            addr+= PAGE_SIZE;
        }

        KernPageTable(page_table)
    }
}

pub enum PagePerm {
    RW,
    RO,
    RWX,
}

pub struct UserPageTable(Box<PageTable>);

impl UserPageTable {
    /// Returns a new `UserPageTable` containing a `PageTable` created with
    /// `USER_RW` permission.
    pub fn new() -> UserPageTable {
        let page_table = PageTable::new(aarch64::EntryPerm::USER_RW);
        UserPageTable(page_table)
    }

    /// Allocates a page and set an L3 entry translates given virtual address to the
    /// physical address of the allocated page. Returns the allocated page.
    ///
    /// # Panics
    /// Panics if the virtual address is lower than `USER_IMG_BASE`.
    /// Panics if the virtual address has already been allocated.
    /// Panics if allocator fails to allocate a page.
    ///
    /// TODO. use Result<T> and make it failurable
    /// TODO. use perm properly
    pub fn alloc(&mut self, va: VirtualAddr, _perm: PagePerm) -> &mut [u8] {
        use core::ops::Sub;
        if va.as_usize() < USER_IMG_BASE {
            panic!("Virtual Address {:?} is lower than USER_IMG_BASE {:?}", va, USER_IMG_BASE);
        }
        if self.is_valid(va.sub(VirtualAddr::from(USER_IMG_BASE))) {
            panic!("Virtual Address {:?} is already allocated", va);
        }
        let addr = unsafe {ALLOCATOR.alloc(Page::layout()) };
        if addr == core::ptr::null_mut() {
            panic!("Allocation failed");
        }
        let saddr = (addr as u64)>>16;
        let mut l3_entry = RawL3Entry::new(0);
        l3_entry.set_value(EntryValid::Valid, RawL3Entry::VALID);
        l3_entry.set_value(PageType::Page, RawL3Entry::TYPE);
        l3_entry.set_value(EntryAttr::Mem, RawL3Entry::ATTR);
        l3_entry.set_value(EntryPerm::USER_RW, RawL3Entry::AP);
        l3_entry.set_value(EntrySh::ISh, RawL3Entry::SH);
        l3_entry.set_value(1, RawL3Entry::AF);
        l3_entry.set_value(saddr, RawL3Entry::ADDR);
        self.set_entry(va.sub(VirtualAddr::from(USER_IMG_BASE)), l3_entry);

        unsafe { core::slice::from_raw_parts_mut(addr, PAGE_SIZE) }


    }
}

impl Deref for KernPageTable {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for UserPageTable {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KernPageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DerefMut for UserPageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

//Implement `Drop` for `UserPageTable`.
impl Drop for UserPageTable {
    fn drop(&mut self) {
        for l3_entry in self.into_iter() {
            if l3_entry.0.get_value(EntryValid::Valid) == RawL3Entry::VALID {
                let addr = l3_entry.0.get_value(RawL3Entry::ADDR) ;
                let addr = addr<<16;
                let addr = addr as *mut u8;
                unsafe {ALLOCATOR.dealloc(addr, Page::layout()) };
            }
        }
    }
}
// FIXME: Implement `fmt::Debug` as you need.
impl fmt::Debug for UserPageTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "User Page Table") 
    }
}

