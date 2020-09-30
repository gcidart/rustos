use core::fmt;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct TrapFrame {
    pub spsr_el1: u64,
    pub elr_el1: u64,
    pub tpidr_el0: u64,
    pub sp_el0: u64,
    pub q: [u128; 32],
    pub x: [u64; 31],
    pub xzr: u64,
}

