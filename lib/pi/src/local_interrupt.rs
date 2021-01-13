use core::time::Duration;

use volatile::prelude::*;
use volatile::Volatile;

const INT_BASE: usize = 0x40000000;

/// Core interrupt sources (QA7: 4.10)
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LocalInterrupt {
    // Lab 5 1.C
    CNTPSIRQ = 0,
    CNTPNSIRQ = 1,
    CNTHPIRQ = 2,
    CNTVIRQ = 3,
    Mailbox0 = 4,
    Mailbox1 = 5,
    Mailbox2 = 6,
    Mailbox3 = 7,
    GPU = 8,
    PMU = 9,
    AXI = 10,
    LocalTimer = 11
}

impl LocalInterrupt {
    pub const MAX: usize = 12;

    pub fn iter() -> impl Iterator<Item = LocalInterrupt> {
        (0..LocalInterrupt::MAX).map(|n| LocalInterrupt::from(n))
    }
}

impl From<usize> for LocalInterrupt {
    fn from(irq: usize) -> LocalInterrupt {
        // Lab 5 1.C
        use LocalInterrupt::*;
        match irq {
            0 =>  CNTPSIRQ,
            1 => CNTPNSIRQ,
            2 => CNTHPIRQ,
            3 => CNTVIRQ,
            4 => Mailbox0,
            5 => Mailbox1,
            6 => Mailbox2,
            7 => Mailbox3,
            9 => GPU,
            9 => PMU,
            10 => AXI,
            11 => LocalTimer,
            _ => panic!("Unknown irq: {}", irq),
        }
    }
}

/// BCM2837 Local Peripheral Registers (QA7: Chapter 4)
#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    // Lab 5 1.C
    control_register: Volatile<u32>,
    unused1: Volatile<u32>,
    core_timer_prescaler: Volatile<u32>,
    gpu_interrupts_polling: Volatile<u32>,
    perfmon_interrupts_routing_set: Volatile<u32>,
    perfmon_interrupts_routing_clear: Volatile<u32>,
    unused2: Volatile<u32>,
    core_timer_access_ls: Volatile<u32>,
    core_timer_access_ms: Volatile<u32>,
    local_interrupt0: Volatile<u32>,
    unused3: Volatile<u32>,
    axi_outstanding_counters: Volatile<u32>,
    axi_outstanding_irq: Volatile<u32>,
    local_timer_control_status: Volatile<u32>,
    local_timer_write_flags: Volatile<u32>,
    unused4: Volatile<u32>,
    core_timers_interrupt_control: [Volatile<u32>; 4],
    core_mailboxes_interrupt_control: [Volatile<u32>; 4],
    core_irq_source: [Volatile<u32>; 4],
    core_fiq_source: [Volatile<u32>; 4],
}

pub struct LocalController {
    core: usize,
    registers: &'static mut Registers,
}

impl LocalController {
    /// Returns a new handle to the interrupt controller.
    pub fn new(core: usize) -> LocalController {
        LocalController {
            core: core,
            registers: unsafe { &mut *(INT_BASE as *mut Registers) },
        }
    }

    pub fn enable_local_timer(&mut self) {
        // Lab 5 1.C
        use aarch64::regs::CNTP_CTL_EL0;
        unsafe {
            CNTP_CTL_EL0.set(1);
        }
        self.registers.core_timers_interrupt_control[self.core].or_mask(2);
    }

    pub fn is_pending(&self, int: LocalInterrupt) -> bool {
        // Lab 5 1.C
        self.registers.core_irq_source[self.core].has_mask(int as u32)
    }

    pub fn tick_in(&mut self, t: Duration) {
        // Lab 5 1.C
        // See timer: 3.1 to 3.3
        let freq = unsafe { aarch64::regs::CNTFRQ_EL0.get() }; 
        let freq = freq/1000000 ;
        let cnt = (t.as_micros() as u64) * freq;
        unsafe {
            aarch64::regs::CNTP_TVAL_EL0.set(cnt);
        }
        self.registers.control_register.write(0);
        self.registers.core_timer_prescaler.write(0x8000_0000);
    }
}

pub fn local_tick_in(core: usize, t: Duration) {
    LocalController::new(core).tick_in(t);
}
