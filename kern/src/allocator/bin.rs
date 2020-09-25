use core::alloc::Layout;
use core::fmt;
use core::ptr;

use crate::allocator::linked_list::LinkedList;
use crate::allocator::util::*;
use crate::allocator::LocalAlloc;

/// A simple allocator that allocates based on size classes.
///   bin 0 (2^3 bytes)    : handles allocations in (0, 2^3]
///   bin 1 (2^4 bytes)    : handles allocations in (2^3, 2^4]
///   ...
///   bin 29 (2^22 bytes): handles allocations in (2^31, 2^32]
///   
///   map_to_bin(size) -> k
///   
const NUM_BINS: usize = 11; /*Corresponds to 8192 bytes*/
/// Returns the bin number for the layout provided  
fn bin_index(layout: Layout) -> usize {
    /// Size of the memory to be allocated is maximum of requested size and 
    /// requested alignment
    let mut size_req = layout.size();
    if layout.align() > size_req {
        size_req = layout.align();
    }
    let mut idx = 0;
    let mut bin_size = 8;
    while idx < NUM_BINS-1 && size_req > bin_size {
        idx+=1;
        bin_size*=2;
    }
    return idx;
}
pub struct Allocator {
    current: usize,
    end: usize,
    bins: [LinkedList; NUM_BINS],
}

impl Allocator {
    /// Creates a new bin allocator that will allocate memory from the region
    /// starting at address `start` and ending at address `end`.
    pub fn new(start: usize, end: usize) -> Allocator {
        Allocator {
            bins: [LinkedList::new(); NUM_BINS],
            current: start,
            end: end,
        }
    }
}

impl LocalAlloc for Allocator {
    /// Allocates memory. Returns a pointer meeting the size and alignment
    /// properties of `layout.size()` and `layout.align()`.
    ///
    /// If this method returns an `Ok(addr)`, `addr` will be non-null address
    /// pointing to a block of storage suitable for holding an instance of
    /// `layout`. In particular, the block will be at least `layout.size()`
    /// bytes large and will be aligned to `layout.align()`. The returned block
    /// of storage may or may not have its contents initialized or zeroed.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure that `layout.size() > 0` and that
    /// `layout.align()` is a power of two. Parameters not meeting these
    /// conditions may result in undefined behavior.
    ///
    /// # Errors
    ///
    /// Returning null pointer (`core::ptr::null_mut`)
    /// indicates that either memory is exhausted
    /// or `layout` does not meet this allocator's
    /// size or alignment constraints.
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        if layout.size() <=0 || layout.align().count_ones() > 1 {
            return core::ptr::null_mut();
        }
        let bidx = bin_index(layout);
        match self.bins[bidx].pop() {
            Some(ptr) => ptr as *mut u8,
            None => {
                let mut bidxc = bidx;
                let mut size_req = 8;
                while bidxc > 0 {
                    size_req*= 2;
                    bidxc-= 1;
                }
                let orig = self.current;
                self.current = align_up(self.current, size_req);
                let start = self.current;
                self.current = self.current.saturating_add(size_req);
                if self.current > self.end {
                    self.current = orig;
                    return core::ptr::null_mut();
                } else {
                    /* Reduce Fragmentation because of alignment*/
                    if(start - orig > self.end - self.current)
                    {
                        self.end = self.current-1;
                        self.current = orig;
                    }
                    return start as *mut u8;
                }
            }
        }
    }

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure the following:
    ///
    ///   * `ptr` must denote a block of memory currently allocated via this
    ///     allocator
    ///   * `layout` must properly represent the original layout used in the
    ///     allocation call that returned `ptr`
    ///
    /// Parameters not meeting these conditions may result in undefined
    /// behavior.
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let bidx = bin_index(layout);
        self.bins[bidx].push(ptr as *mut usize);
    }
}

impl fmt::Debug for Allocator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Allocator")
            .field("current", &self.current)
            .field("end", &self.end)
            .field("bins[0]", &self.bins[0])
            .field("bins[1]", &self.bins[1])
            .field("bins[2]", &self.bins[2])
            .field("bins[3]", &self.bins[3])
            .field("bins[4]", &self.bins[4])
            .field("bins[5]", &self.bins[5])
            .field("bins[6]", &self.bins[6])
            .field("bins[7]", &self.bins[7])
            .field("bins[8]", &self.bins[8])
            .field("bins[9]", &self.bins[9])
            .field("bins[10]", &self.bins[10])
            .finish()
    }
}

