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
            //crate::console::kprintln!("{:?} > {:?} + {:?}", curr_time, ini_time, sleep_dur);
            true 
        } else {
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
    use pi::timer::current_time;
    let curr_time = current_time();
    tf.x[7] = 1;
    tf.x[0] = curr_time.as_secs();
    tf.x[1] = curr_time.subsec_nanos().into();
}

/// Kills current process.
///
/// This system call does not take paramer and does not return any value.
pub fn sys_exit(tf: &mut TrapFrame) {
    //Can't use SCHEDULER.kill(tf) as the user programs effectively have a empty loop after NR_EXIT
    //call. Kill() drops the process pagtables which causes Instruction Abort for loop statement
    SCHEDULER.switch(State::Dead, tf);
}

/// Write to console.
///
/// This system call takes one parameter: a u8 character to print.
///
/// It only returns the usual status value.
pub fn sys_write(b: u8, tf: &mut TrapFrame) {
    tf.x[7] = 1;
    CONSOLE.lock().write_byte(b);
}

/// Returns current process's ID.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns a
/// parameter: the current process's ID.
pub fn sys_getpid(tf: &mut TrapFrame) {
    tf.x[7] = 1;
    tf.x[0] = tf.tpidr_el0;
}

pub fn handle_syscall(num: u16, tf: &mut TrapFrame) {
    match num as usize{
        NR_SLEEP => sys_sleep(tf.x[0] as u32, tf),
        NR_TIME => sys_time(tf),
        NR_EXIT => sys_exit(tf),
        NR_WRITE => sys_write(tf.x[0] as u8, tf),
        NR_GETPID => sys_getpid(tf),
        _ => {}
    }
}
