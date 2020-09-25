/// Align `addr` downwards to the nearest multiple of `align`.
///
/// The returned usize is always <= `addr.`
///
/// # Panics
///
/// Panics if `align` is not a power of 2.
pub fn align_down(addr: usize, align: usize) -> usize {
    if align.count_ones() > 1 {
        panic!();
    }
    let rem = addr%align;
    let aligned_addr = addr - rem;
    return aligned_addr;
}

/// Align `addr` upwards to the nearest multiple of `align`.
///
/// The returned `usize` is always >= `addr.`
///
/// # Panics
///
/// Panics if `align` is not a power of 2
/// or aligning up overflows the address.
pub fn align_up(addr: usize, align: usize) -> usize {
    if align.count_ones() > 1 {
        panic!();
    }
    let rem = addr%align;
    let mut aligned_addr = addr;
    if rem !=0 {
        aligned_addr += align - rem;
    }
    if aligned_addr < addr {
        panic!();
    }
    return aligned_addr;
}
