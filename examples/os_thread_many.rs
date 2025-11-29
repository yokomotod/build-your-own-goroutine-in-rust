use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let start = Instant::now();
    println!(
        "[{:>8.3}s] Spawning many tasks with OS threads...",
        start.elapsed().as_secs_f64()
    );

    let task_count = 100_000;
    let mut handles = vec![];

    for i in 0..task_count {
        let handle = thread::spawn(move || {
            // Simple computation
            let _ = i * 2;
            thread::sleep(Duration::from_secs(10));
        });
        handles.push(handle);

        if (i + 1) % 1000 == 0 {
            println!(
                "[{:>8.3}s] Spawned {} threads...",
                start.elapsed().as_secs_f64(),
                i + 1
            );
        }
    }

    println!(
        "[{:>8.3}s] Waiting for all threads to complete...",
        start.elapsed().as_secs_f64()
    );
    for handle in handles {
        handle.join().unwrap();
    }

    println!("[{:>8.3}s] Done!", start.elapsed().as_secs_f64());
}
