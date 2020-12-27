use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use core::fmt;

use aarch64::*;

use crate::mutex::Mutex;
use crate::param::{PAGE_MASK, PAGE_SIZE, TICK, USER_IMG_BASE};
use crate::process::{Id, Process, State};
use crate::traps::TrapFrame;
use crate::VMM;

/// Process scheduler for the entire machine.
#[derive(Debug)]
pub struct GlobalScheduler(Mutex<Option<Scheduler>>);

impl GlobalScheduler {
    /// Returns an uninitialized wrapper around a local scheduler.
    pub const fn uninitialized() -> GlobalScheduler {
        GlobalScheduler(Mutex::new(None))
    }

    /// Enter a critical region and execute the provided closure with the
    /// internal scheduler.
    pub fn critical<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Scheduler) -> R,
    {
        let mut guard = self.0.lock();
        f(guard.as_mut().expect("scheduler uninitialized"))
    }


    /// Adds a process to the scheduler's queue and returns that process's ID.
    /// For more details, see the documentation on `Scheduler::add()`.
    pub fn add(&self, process: Process) -> Option<Id> {
        self.critical(move |scheduler| scheduler.add(process))
    }

    /// Performs a context switch using `tf` by setting the state of the current
    /// process to `new_state`, saving `tf` into the current process, and
    /// restoring the next process's trap frame into `tf`. For more details, see
    /// the documentation on `Scheduler::schedule_out()` and `Scheduler::switch_to()`.
    pub fn switch(&self, new_state: State, tf: &mut TrapFrame) -> Id {
        self.critical(|scheduler| scheduler.schedule_out(new_state, tf));
        self.switch_to(tf)
    }

    pub fn switch_to(&self, tf: &mut TrapFrame) -> Id {
        loop {
            let rtn = self.critical(|scheduler| scheduler.switch_to(tf));
            if let Some(id) = rtn {
                return id;
            }
            aarch64::wfe();
        }
    }

    /// Kills currently running process and returns that process's ID.
    /// For more details, see the documentaion on `Scheduler::kill()`.
    #[must_use]
    pub fn kill(&self, tf: &mut TrapFrame) -> Option<Id> {
        self.critical(|scheduler| scheduler.kill(tf))
    }


    /// Starts executing processes in user space using timer interrupt based
    /// preemptive scheduling. This method should not return under normal conditions.
    pub fn start(&self) -> ! {
        use pi::timer::tick_in;
        use pi::interrupt::{Controller, Interrupt};
        crate::IRQ.register(Interrupt::Timer1, Box::new(timer1_handler));
        let mut controller = Controller::new();
        controller.enable(Interrupt::Timer1);
        tick_in(TICK);
        let sptr: u64; 
        unsafe {
            asm!("mov $0, sp"  //Store Current Stack Pointer to sptr            
                 : "=r"(sptr) ::: "volatile");
        }
        let mut frame : TrapFrame = TrapFrame::default();
        self.switch_to(&mut frame);
        let fptr = &frame as *const _ as u64;
        /*use crate::process::Stack;
        let st = Stack::new().unwrap();
        frame.sp_el0 = st.top().as_u64(); 
        frame.elr_el1 = crate::run_shell as *const() as u64;*/
        //Stored in frame.x[27] as after context restore original SP will be available in reg x27
        frame.x[27] = sptr; 
        unsafe {
            asm!("mov sp, $0
                  bl context_restore"
                  :: "r"(fptr)
                  :: "volatile");
        }
        // Restore SP to original value and clear x27 to avoid leaking info to user level process
        unsafe {
            asm!("mov sp, x27
                  mov x27, #0    
                  eret"
                  :::: "volatile");
        }
        loop {
        }
    }

    /// Initializes the scheduler and add userspace processes to the Scheduler
    pub unsafe fn initialize(&self) {
        use shim::path::Path;
        *self.0.lock() = Some(Scheduler::new());
        let mut process1  = Process::new().unwrap();
        //process1.context.elr_el1 = crate::run_shell as *const() as u64;
        /*process1.context.elr_el1 = USER_IMG_BASE as u64;
        process1.context.ttbr0_el1 = VMM.get_baddr().as_u64();
        process1.context.ttbr1_el1 = process1.vmap.as_ref().get_baddr().as_u64();
        process1.context.sp_el0 = process1.stack.top().as_u64();
        self.test_phase_3(&mut process1);*/
        let process1 = Process::load(Path::new("/sleep")).unwrap();
        self.add(process1);
        //let mut process2  = Process::new().unwrap();
        //process2.context.elr_el1 = crate::run_shell_dup as *const() as u64;
        /*process2.context.elr_el1 = USER_IMG_BASE as u64;
        process2.context.ttbr0_el1 = VMM.get_baddr().as_u64();
        process2.context.ttbr1_el1 = process2.vmap.as_ref().get_baddr().as_u64();
        process2.context.sp_el0 = process2.stack.top().as_u64();
        self.test_phase_3(&mut process2);*/
        let process2 = Process::load(Path::new("/sleep")).unwrap();
        self.add(process2);
        let process3 = Process::load(Path::new("/sleep")).unwrap();
        self.add(process3);
        let process4 = Process::load(Path::new("/sleep")).unwrap();
        self.add(process4);
    }

    // The following method may be useful for testing Phase 3:
    //
    // * A method to load a extern function to the user process's page table.
    //
    pub fn test_phase_3(&self, proc: &mut Process){
        use crate::vm::{VirtualAddr, PagePerm};
    
        let mut page = proc.vmap.alloc(
            VirtualAddr::from(USER_IMG_BASE as u64), PagePerm::RWX);
   
        let text = unsafe {
            core::slice::from_raw_parts(test_user_process as *const u8, 24)
        };
    
        page[0..24].copy_from_slice(text);
    }
}

#[derive(Debug)]
pub struct Scheduler {
    processes: VecDeque<Process>,
    last_id: Option<Id>,
}

impl Scheduler {
    /// Returns a new `Scheduler` with an empty queue.
    fn new() -> Scheduler {
        Scheduler {
            processes: VecDeque::new(),
            last_id : Some(0),
        }
    }

    /// Adds a process to the scheduler's queue and returns that process's ID if
    /// a new process can be scheduled. The process ID is newly allocated for
    /// the process and saved in its `trap_frame`. If no further processes can
    /// be scheduled, returns `None`.
    ///
    /// It is the caller's responsibility to ensure that the first time `switch`
    /// is called, that process is executing on the CPU.
    fn add(&mut self, mut process: Process) -> Option<Id> {
        match self.last_id {
            None => {
                self.last_id = Some(1u64);
            }
            Some(id) => {
                match id.checked_add(1) {
                    None => { return None; },
                    Some(new_id) => {self.last_id = Some(new_id); },
                }
            }
        }
        process.context.tpidr_el0 = self.last_id.unwrap();
        self.processes.push_back(process);
        return self.last_id;
    }

    /// Finds the currently running process, sets the current process's state
    /// to `new_state`, prepares the context switch on `tf` by saving `tf`
    /// into the current process, and push the current process back to the
    /// end of `processes` queue.
    ///
    /// If the `processes` queue is empty or there is no current process,
    /// returns `false`. Otherwise, returns `true`.
    fn schedule_out(&mut self, new_state: State, tf: &mut TrapFrame) -> bool {
        let mut idx = 0;
        for process in self.processes.iter() {
            if process.context.tpidr_el0 == tf.tpidr_el0 {
                match process.state {
                    State::Running => { 
                        crate::console::kprintln!("Process{:?} scheduled out with new state {:?}", tf.tpidr_el0, new_state);
                        break; 
                    },
                    _ => { idx = idx + 1; }
                }
            }
            else {
                idx = idx + 1;
            }
        }
        if self.processes.len() == idx {
            return false;
        }
        let mut current_process = self.processes.remove(idx).unwrap();
        current_process.state = new_state;
        *(current_process.context) = *tf;
        self.processes.push_back(current_process);
        return true;
    }

    /// Finds the next process to switch to, brings the next process to the
    /// front of the `processes` queue, changes the next process's state to
    /// `Running`, and performs context switch by restoring the next process`s
    /// trap frame into `tf`.
    ///
    /// If there is no process to switch to, returns `None`. Otherwise, returns
    /// `Some` of the next process`s process ID.
    fn switch_to(&mut self, tf: &mut TrapFrame) -> Option<Id> {
        let mut idx = 0;
        for process in self.processes.iter_mut() {
            if process.is_ready() {
                crate::console::kprintln!("Process{:?} now running", process.context.tpidr_el0);
                break;
            }
            else {
                idx = idx + 1;
            }
        }
        if self.processes.len() == idx {
            return None;
        }
        let mut next_process = self.processes.remove(idx).unwrap();
        *tf = *(next_process.context);
        next_process.state = State::Running;
        self.processes.push_front(next_process);
        return Some(self.processes.front().unwrap().context.tpidr_el0);
    }

    /// Kills currently running process by scheduling out the current process
    /// as `Dead` state. Removes the dead process from the queue, drop the
    /// dead process's instance, and returns the dead process's process ID.
    fn kill(&mut self, tf: &mut TrapFrame) -> Option<Id> {
        self.schedule_out(State::Dead, tf);
        match self.processes.pop_back() {
            Some(process) => {
                let pid = process.context.tpidr_el0;
                //drop(process);
                Some(pid)
            },
            None => None,
        }
    }
}

pub extern "C" fn  test_user_process() -> ! {
    loop {
        let ms = 10000;
        let error: u64;
        let elapsed_ms: u64;

        unsafe {
            asm!("mov x0, $2
              svc 1
              mov $0, x0
              mov $1, x7"
                 : "=r"(elapsed_ms), "=r"(error)
                 : "r"(ms)
                 : "x0", "x7"
                 : "volatile");
        }
    }
}

pub fn timer1_handler(tf: &mut TrapFrame) {
    crate::console::kprintln!("Timer interrupt after {:?}", TICK);
    pi::timer::tick_in(TICK);
    crate::SCHEDULER.switch(State::Ready, tf);
}
