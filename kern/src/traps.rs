mod frame;
mod syndrome;
mod syscall;

pub mod irq;
pub use self::frame::TrapFrame;

use pi::interrupt::{Controller, Interrupt};
use crate::shell;
use pi::local_interrupt::{LocalController, LocalInterrupt};

use self::syndrome::Syndrome;
use self::syscall::handle_syscall;
use crate::percore;
use crate::traps::irq::IrqHandlerRegistry;

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,
}

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Source {
    CurrentSpEl0 = 0,
    CurrentSpElx = 1,
    LowerAArch64 = 2,
    LowerAArch32 = 3,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Info {
    source: Source,
    kind: Kind,
}

/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
#[no_mangle]
pub extern "C" fn handle_exception(info: Info, esr: u32, tf: &mut TrapFrame) {
    use crate::console::kprintln;

    if info.kind == Kind::Irq {
        /*if aarch64::affinity()==0 {
            crate::GLOBAL_IRQ.invoke(Interrupt::Timer1, tf);
        }*/
        percore::local_irq().invoke(LocalInterrupt::CNTPNSIRQ, tf);
        return;
    }
        
    match Syndrome::from(esr) {
        Syndrome::Brk(x) => {
            kprintln!("Brk{:?} encountered", x);
            shell::shell("Debug# ");
            tf.elr_el1+= 4;  // Synchronous Exception
            kprintln!("Debug shell exited");
        },
        Syndrome::Svc(y) => {
            //trace!("Svc{:?} encountered", y);
            handle_syscall(y, tf);
        },
        Syndrome::DataAbort {
            kind:x, level: y
            }=> {
            kprintln!("DataAbort encountered Kind:{:?} Level:{:?}", x, y);
        },

        _      =>  {
            kprintln!("Info: {:?} ESR: {:?}", info, esr);
        },
    }
}

