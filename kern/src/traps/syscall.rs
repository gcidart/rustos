use alloc::boxed::Box;
use core::time::Duration;

use crate::console::CONSOLE;
use crate::process::{State, Process};
use crate::traps::TrapFrame;
use crate::SCHEDULER;
use kernel_api::*;

/// Sleep for `ms` milliseconds.
///
/// This system call takes one parameter: the number of milliseconds to sleep.
///
/// In addition to the usual status value, this system call returns one
/// parameter: the approximate true elapsed time from when `sleep` was called to
/// when `sleep` returned.
pub fn sys_sleep(ms: u32, tf: &mut TrapFrame) {
    use pi::timer::current_time;
    let ini_time = current_time();
    let sleep_dur = Duration::from_millis(ms as u64);
    let sleep_fn = Box::new(move |process: &mut Process| -> bool {
        let curr_time = current_time();
        process.context.x[7] = 1;
        process.context.x[0] = (curr_time.as_millis() - ini_time.as_millis()) as u64;
        if curr_time > ini_time + sleep_dur {
            crate::console::kprintln!("{:?} > {:?} + {:?}", curr_time, ini_time, sleep_dur);
            true 
        } else {
            crate::console::kprintln!("{:?} < {:?} + {:?}", curr_time, ini_time, sleep_dur);
            false
        }
    });

    SCHEDULER.switch(State::Waiting(sleep_fn), tf);
}

/// Returns current time.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns two
/// parameter:
///  - current time as seconds
///  - fractional part of the current time, in nanoseconds.
pub fn sys_time(tf: &mut TrapFrame) {
    unimplemented!("sys_time()");
}

/// Kills current process.
///
/// This system call does not take paramer and does not return any value.
pub fn sys_exit(tf: &mut TrapFrame) {
    unimplemented!("sys_exit()");
}

/// Write to console.
///
/// This system call takes one parameter: a u8 character to print.
///
/// It only returns the usual status value.
pub fn sys_write(b: u8, tf: &mut TrapFrame) {
    unimplemented!("sys_write()");
}

/// Returns current process's ID.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns a
/// parameter: the current process's ID.
pub fn sys_getpid(tf: &mut TrapFrame) {
    unimplemented!("sys_getpid()");
}

pub fn handle_syscall(num: u16, tf: &mut TrapFrame) {
    use crate::console::kprintln;
    match num {
        1 => sys_sleep(tf.x[0] as u32, tf),
        _ => {}
    }
}
