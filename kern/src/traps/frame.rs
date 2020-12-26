use core::fmt;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct TrapFrame {
    pub elr_el1: u64,
    pub spsr_el1: u64,
    pub sp_el0: u64,
    pub tpidr_el0: u64,
    pub ttbr0_el1: u64,
    pub ttbr1_el1: u64,
    pub q: [u128; 32],
    pub x: [u64; 30],
    pub xzr: u64,
}

