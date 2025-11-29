//! N:1 Green Thread Runtime with hand-written context switch
//!
//! This module implements a simple N:1 (many-to-one) green thread runtime
//! using inline assembly for context switching.

use std::arch::{asm, naked_asm};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ptr;

/// Stack size for each green thread (64KB)
const STACK_SIZE: usize = 64 * 1024;

/// Saved CPU context for context switching
///
/// On x86_64 System V ABI, these are the callee-saved registers
/// that must be preserved across function calls.
#[repr(C)]
#[derive(Debug, Clone, Default)]
struct Context {
    // Callee-saved registers (must be preserved)
    rsp: u64, // Stack pointer
    rbp: u64, // Frame pointer
    rbx: u64, // General purpose
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
}

/// A green thread task
struct Task {
    context: Context,
    #[allow(dead_code)]
    stack: Vec<u8>, // Keep stack alive
    finished: bool,
}

impl Task {
    /// Create a new task with the given entry function
    fn new<F>(f: F) -> Self
    where
        F: FnOnce() + 'static,
    {
        let mut stack = vec![0u8; STACK_SIZE];

        // Stack grows downward, so we start at the top
        let stack_top = stack.as_mut_ptr() as usize + STACK_SIZE;

        // Align stack to 16 bytes (required by System V ABI)
        // After our setup, when task_entry is called, RSP should be 16-byte aligned
        let stack_top = stack_top & !0xF;

        // Box the closure and leak it to get a raw pointer
        let f_ptr = Box::into_raw(Box::new(f));

        // Set up initial stack:
        // The context_switch function will do `ret` which pops the return address.
        // We want it to jump to task_entry.
        //
        // System V ABI requires RSP to be 16-byte aligned BEFORE `call` instruction.
        // After `call`, RSP becomes 16n+8 (due to pushed return address).
        // Since we use `ret` instead of `call`, we need to simulate this:
        //
        // Stack layout (growing downward):
        //   stack_top - 8:  (padding for alignment)
        //   stack_top - 16: return address (task_entry)
        //
        // After `ret`: RSP = stack_top - 8, which is 16n+8 as required.

        let initial_rsp = stack_top - 16;

        unsafe {
            // Write task_entry as the return address
            ptr::write(initial_rsp as *mut u64, task_entry::<F> as usize as u64);
        }

        let context = Context {
            rsp: initial_rsp as u64,
            r15: f_ptr as u64, // Pass closure pointer via r15
            ..Default::default()
        };

        Task {
            context,
            stack,
            finished: false,
        }
    }
}

/// Entry point for new tasks
///
/// This function is called when a task is first resumed.
/// The closure pointer is passed in r15.
extern "C" fn task_entry<F>()
where
    F: FnOnce() + 'static,
{
    unsafe {
        // Get the closure pointer from r15
        let f_ptr: u64;
        asm!(
            "mov {}, r15",
            out(reg) f_ptr,
            options(nomem, nostack, preserves_flags)
        );

        // Take ownership of the closure and run it
        let f = Box::from_raw(f_ptr as *mut F);
        f();
    }

    // Task finished - mark as done and yield back
    task_finished();
}

/// Called when a task completes
fn task_finished() {
    // Get the runtime pointer and release the borrow immediately
    // (before context switch, which never returns to this stack frame)
    let runtime_ptr = RUNTIME.with(|rt| *rt.borrow());

    if let Some(runtime) = runtime_ptr {
        unsafe {
            (*runtime).current_task_finished = true;
            (*runtime).switch_to_scheduler();
        }
    }
}

/// Switch from one context to another
///
/// Saves the current CPU state into `old` and restores state from `new`.
/// This function returns when another context switches back to `old`.
#[unsafe(naked)]
extern "C" fn context_switch(old: *mut Context, new: *const Context) {
    naked_asm!(
        // Save callee-saved registers to old context
        "mov [rdi + 0x00], rsp",
        "mov [rdi + 0x08], rbp",
        "mov [rdi + 0x10], rbx",
        "mov [rdi + 0x18], r12",
        "mov [rdi + 0x20], r13",
        "mov [rdi + 0x28], r14",
        "mov [rdi + 0x30], r15",
        // Load callee-saved registers from new context
        "mov rsp, [rsi + 0x00]",
        "mov rbp, [rsi + 0x08]",
        "mov rbx, [rsi + 0x10]",
        "mov r12, [rsi + 0x18]",
        "mov r13, [rsi + 0x20]",
        "mov r14, [rsi + 0x28]",
        "mov r15, [rsi + 0x30]",
        // Return to the new context
        // For a fresh task: pops task_entry address and jumps there
        // For a yielded task: returns to where it called context_switch
        "ret",
    );
}

thread_local! {
    static RUNTIME: RefCell<Option<*mut Runtime>> = const { RefCell::new(None) };
}

/// N:1 Green Thread Runtime
pub struct Runtime {
    /// Queue of runnable tasks
    tasks: VecDeque<Task>,
    /// Context to return to when a task yields
    scheduler_context: Context,
    /// The currently running task (moved out of queue during execution)
    current_task: Option<Task>,
    /// Flag set by task_finished()
    current_task_finished: bool,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            tasks: VecDeque::new(),
            scheduler_context: Context::default(),
            current_task: None,
            current_task_finished: false,
        }
    }

    /// Spawn a new green thread
    pub fn go<F>(&mut self, f: F)
    where
        F: FnOnce() + 'static,
    {
        let task = Task::new(f);
        self.tasks.push_back(task);
    }

    /// Run the scheduler until all tasks complete
    pub fn run(&mut self) {
        // Register this runtime in thread-local storage
        RUNTIME.with(|rt| {
            *rt.borrow_mut() = Some(self as *mut Runtime);
        });

        loop {
            // Get the next task
            let Some(task) = self.tasks.pop_front() else {
                break;
            };

            if task.finished {
                continue;
            }

            // Move task to current_task
            self.current_task = Some(task);
            self.current_task_finished = false;

            // Switch to the task
            let task_ctx = &self.current_task.as_ref().unwrap().context as *const Context;
            context_switch(&mut self.scheduler_context, task_ctx);

            // We're back! Task either yielded or finished
            if let Some(mut task) = self.current_task.take() {
                if self.current_task_finished {
                    task.finished = true;
                    // Task is dropped here
                } else {
                    // Task yielded, put it back in the queue
                    self.tasks.push_back(task);
                }
            }
        }

        // Cleanup
        RUNTIME.with(|rt| {
            *rt.borrow_mut() = None;
        });
    }

    /// Switch from current task back to scheduler
    unsafe fn switch_to_scheduler(&mut self) {
        if let Some(ref mut task) = self.current_task {
            context_switch(&mut task.context, &self.scheduler_context);
        }
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

/// Yield execution to another green thread
pub fn gosched() {
    RUNTIME.with(|rt| {
        let rt_ptr = rt.borrow();
        if let Some(runtime) = rt_ptr.as_ref() {
            unsafe {
                (**runtime).switch_to_scheduler();
            }
        }
    });
}
