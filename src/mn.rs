use corosensei::{Coroutine, CoroutineResult, Yielder};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;

thread_local! {
    static CURRENT_YIELDER: RefCell<Option<*const Yielder<(), ()>>> = const { RefCell::new(None) };
}

type Task = Coroutine<(), (), ()>;
type TaskFn = Box<dyn FnOnce() + Send + 'static>;

/// Shared state for the M:N runtime
struct SharedQueue {
    /// Queue of pending task functions (not yet turned into coroutines)
    pending: VecDeque<TaskFn>,
    /// Flag to signal shutdown
    shutdown: bool,
}

pub struct Runtime {
    num_threads: usize,
    shared: Arc<Mutex<SharedQueue>>,
}

impl Runtime {
    pub fn new(num_threads: usize) -> Self {
        Runtime {
            num_threads,
            shared: Arc::new(Mutex::new(SharedQueue {
                pending: VecDeque::new(),
                shutdown: false,
            })),
        }
    }

    pub fn go<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let mut queue = self.shared.lock().unwrap();
        queue.pending.push_back(Box::new(f));
    }

    pub fn run(&self) {
        let mut handles = Vec::new();

        // Spawn worker threads
        for worker_id in 0..self.num_threads {
            let shared = Arc::clone(&self.shared);
            let handle = thread::spawn(move || {
                worker_loop(worker_id, shared);
            });
            handles.push(handle);
        }

        // Wait for all workers to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }
}

fn worker_loop(worker_id: usize, shared: Arc<Mutex<SharedQueue>>) {
    // Each worker has its own local run queue of active coroutines
    let mut local_tasks: VecDeque<Task> = VecDeque::new();

    loop {
        // First, try to run local tasks
        if let Some(mut task) = local_tasks.pop_front() {
            match task.resume(()) {
                CoroutineResult::Yield(()) => {
                    local_tasks.push_back(task);
                }
                CoroutineResult::Return(()) => {
                    // Task completed
                }
            }
            continue;
        }

        // No local tasks, try to get from shared queue
        let mut queue = shared.lock().unwrap();

        if let Some(task_fn) = queue.pending.pop_front() {
            drop(queue);

            // Create coroutine from task function
            let task = Coroutine::new(move |yielder, ()| {
                CURRENT_YIELDER.with(|cy| {
                    *cy.borrow_mut() = Some(yielder as *const _);
                });
                task_fn();
            });
            local_tasks.push_back(task);
            continue;
        }

        // Pending is empty at this point (pop_front returned None)
        if queue.shutdown {
            // Another worker already initiated shutdown
            break;
        }

        // First worker to find pending empty initiates shutdown
        queue.shutdown = true;
        break;
    }

    println!("[Worker {}] Shutting down", worker_id);
}

pub fn gosched() {
    CURRENT_YIELDER.with(|cy| {
        let yielder_ptr = cy.borrow().expect("gosched called outside of runtime");
        unsafe {
            (*yielder_ptr).suspend(());
        }
    });
}
