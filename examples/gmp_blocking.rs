use mygoroutine::gmp::{Runtime, io};
use std::time::Instant;

fn main() {
    let num_processors = 4; // P: logical processors
    let num_workers = 16; // M: OS threads
    let num_tasks = 32;

    println!(
        "=== GMP Runtime (P={}, M={}) with blocking I/O ===",
        num_processors, num_workers
    );
    println!("Running {} tasks that each block for 100ms", num_tasks);
    println!("Using io::sleep() which releases P during blocking");
    println!();

    let start = Instant::now();
    let runtime = Runtime::new(num_processors, num_workers);

    for i in 0..num_tasks {
        runtime.go(move || {
            println!(
                "[{:>6.3}s] Task {} started",
                start.elapsed().as_secs_f64(),
                i
            );

            // Use GMP-aware sleep that releases P during blocking
            io::sleep(std::time::Duration::from_millis(100));

            println!("[{:>6.3}s] Task {} done", start.elapsed().as_secs_f64(), i);
        });
    }

    runtime.run();

    println!();
    println!("Total elapsed: {:?}", start.elapsed());
    println!();
    println!("M:N (4 threads, no handoff): ~800ms (32 tasks / 4 = 8 rounds)");
    println!("GMP (P=4, M=16):             ~200ms (32 tasks / 16 = 2 rounds)");
}
