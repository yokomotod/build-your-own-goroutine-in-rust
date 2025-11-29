use mygoroutine::gmp::{Runtime, gosched};

fn main() {
    let runtime = Runtime::new(4, 16); // P=4, M=16

    runtime.go(|| {
        println!("Task 1: start");
        gosched();
        println!("Task 1: end");
    });

    runtime.go(|| {
        println!("Task 2: start");
        gosched();
        println!("Task 2: end");
    });

    runtime.go(|| {
        println!("Task 3: start");
        gosched();
        println!("Task 3: end");
    });

    println!("Running GMP scheduler with 4 processors...");
    runtime.run();
    println!("All tasks completed!");
}
