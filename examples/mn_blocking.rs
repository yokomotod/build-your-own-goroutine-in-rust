use mygoroutine::mn::Runtime;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let num_threads = 4;
    let num_tasks = 32;

    println!(
        "=== M:N Runtime ({} threads) with blocking I/O ===",
        num_threads
    );
    println!("Running {} tasks that each block for 100ms", num_tasks);
    println!();

    let start = Instant::now();
    let runtime = Runtime::new(num_threads);

    for i in 0..num_tasks {
        runtime.go(move || {
            println!(
                "[{:>6.3}s] Task {} started",
                start.elapsed().as_secs_f64(),
                i
            );

            // Simulate blocking I/O (e.g., file read, network request)
            thread::sleep(Duration::from_millis(100));

            println!("[{:>6.3}s] Task {} done", start.elapsed().as_secs_f64(), i);
        });
    }

    runtime.run();

    println!();
    println!("Total elapsed: {:?}", start.elapsed());
    println!();
    println!("Expected if no blocking: ~100ms (all tasks run in parallel)");
    println!("Expected with blocking:  ~800ms (4 threads, 32 tasks, 8 rounds)");
    println!("Actual result shows threads are blocked during sleep!");
}
