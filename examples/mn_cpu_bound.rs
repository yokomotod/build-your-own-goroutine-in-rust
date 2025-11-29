use mygoroutine::mn::Runtime;
use std::hint::black_box;
use std::time::Instant;

/// CPU-intensive work: compute sum of squares
fn cpu_work(n: u64) -> u64 {
    let mut sum = 0u64;
    for i in 0..n {
        sum = sum.wrapping_add(black_box(i).wrapping_mul(black_box(i)));
    }
    sum
}

fn main() {
    let work_size = 50_000_000u64;
    let num_tasks = 8;
    let num_threads = 4;

    println!("=== M:N Runtime ({} threads) ===", num_threads);
    println!("Running {} tasks with work_size = {}", num_tasks, work_size);
    println!("Watch CPU usage with: htop or top");
    println!();

    let start = Instant::now();
    let runtime = Runtime::new(num_threads);

    for i in 0..num_tasks {
        runtime.go(move || {
            let result = cpu_work(work_size);
            println!("Task {}: result = {}", i, result);
        });
    }

    runtime.run();
    println!();
    println!("Elapsed: {:?}", start.elapsed());
}
