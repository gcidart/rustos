use crate::common::IO_BASE;

use volatile::prelude::*;
use volatile::{ReadVolatile, Volatile};

const INT_BASE: usize = IO_BASE + 0xB000 + 0x200;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Interrupt {
    Timer1 = 1,
    Timer3 = 3,
    Usb = 9,
    Gpio0 = 49,
    Gpio1 = 50,
    Gpio2 = 51,
    Gpio3 = 52,
    Uart = 57,
}

impl Interrupt {
    pub const MAX: usize = 8;

    pub fn iter() -> impl Iterator<Item = Interrupt> {
        use Interrupt::*;
        [Timer1, Timer3, Usb, Gpio0, Gpio1, Gpio2, Gpio3, Uart]
            .iter()
            .map(|int| *int)
    }
}

impl From<usize> for Interrupt {
    fn from(irq: usize) -> Interrupt {
        use Interrupt::*;
        match irq {
            1 => Timer1,
            3 => Timer3,
            9 => Usb,
            49 => Gpio0,
            50 => Gpio1,
            51 => Gpio2,
            52 => Gpio3,
            57 => Uart,
            _ => panic!("Unknown irq: {}", irq),
        }
    }
}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    IRQ_basic_pending: ReadVolatile<u32>,
    IRQ_pending1: ReadVolatile<u32>,
    IRQ_pending2: ReadVolatile<u32>,
    FIQ_control: Volatile<u32>,
    enable_IRQs1: Volatile<u32>,
    enable_IRQs2: Volatile<u32>,
    enable_basic_IRQs: Volatile<u32>,
    disable_IRQs1: Volatile<u32>,
    disable_IRQs2: Volatile<u32>,
    disable_basic_IRQs: Volatile<u32>
}

/// An interrupt controller. Used to enable and disable interrupts as well as to
/// check if an interrupt is pending.
pub struct Controller {
    registers: &'static mut Registers,
}

impl Controller {
    /// Returns a new handle to the interrupt controller.
    pub fn new() -> Controller {
        Controller {
            registers: unsafe { &mut *(INT_BASE as *mut Registers) },
        }
    }

    /// Enables the interrupt `int`.
    pub fn enable(&mut self, int: Interrupt) {
        let mut int = int as u32;
        if int<32 
        {
            self.registers.enable_IRQs1.or_mask(1<<int);
        }
        else
        {
            int = int-31;
            self.registers.enable_IRQs2.or_mask(1<<int);
        }
    }

    /// Disables the interrupt `int`.
    pub fn disable(&mut self, int: Interrupt) {
        let mut int = int as u32;
        if int<32 
        {
            self.registers.disable_IRQs1.or_mask(1<<int);
        }
        else
        {
            int = int-31;
            self.registers.disable_IRQs2.or_mask(1<<int);
        }
    }

    /// Returns `true` if `int` is pending. Otherwise, returns `false`.
    pub fn is_pending(&self, int: Interrupt) -> bool {
        let mut int = int as u32;
        if int<32 
        {
            self.registers.IRQ_pending1.has_mask(1<<int)
        }
        else
        {
            int = int-31;
            self.registers.IRQ_pending1.has_mask(1<<int)
        }
    }

    /// Enables the interrupt as FIQ interrupt
    pub fn enable_fiq(&mut self, int: Interrupt) {
        // Lab 5 2.B
        unimplemented!("enable_fiq")
    }
}
