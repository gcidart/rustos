#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![feature(ptr_internals)]
#![feature(raw_vec_internals)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

extern crate alloc;

pub mod allocator;
pub mod console;
pub mod fs;
pub mod mutex;
pub mod shell;
pub mod param;
pub mod process;
pub mod traps;
pub mod vm;

use console::kprintln;

use allocator::Allocator;
use pi::timer;
use fs::FileSystem;
use process::GlobalScheduler;
use traps::irq::Irq;
use vm::VMManager;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();
pub static SCHEDULER: GlobalScheduler = GlobalScheduler::uninitialized();
pub static VMM: VMManager = VMManager::uninitialized();
pub static IRQ: Irq = Irq::uninitialized();

extern fn run_shell() {
    unsafe { asm!("brk 1" :::: "volatile"); }
    unsafe { asm!("brk 2" :::: "volatile"); }
    shell::shell("user0> ");
    unsafe { asm!("brk 3" :::: "volatile"); }
    loop { shell::shell("user1> "); }
}

extern fn run_shell_dup() {
    loop { shell::shell("dup> "); }
}

fn kmain() -> ! {
    timer::spin_sleep(core::time::Duration::from_millis(3000));
    use aarch64::current_el;
    unsafe {
        kprintln!("Current Exception Level: {}", current_el());
    }
    unsafe {
       ALLOCATOR.initialize();
       FILESYSTEM.initialize();
       IRQ.initialize();
       SCHEDULER.initialize();
       SCHEDULER.start();
    }
    //Not reachable because of SCHEDULER.start()
    /*kprintln!("Welcome to cs3210!");
    use aarch64::brk;
    use aarch64::svc;
    brk!(12);
    svc!(3);
    loop{
        shell::shell("> ");
    }*/
}

