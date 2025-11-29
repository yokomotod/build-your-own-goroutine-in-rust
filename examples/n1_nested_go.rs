use mygoroutine::n1::{Runtime, go, gosched};
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let mut runtime = Runtime::new();
    let counter = Rc::new(RefCell::new(0));

    // Spawn initial task
    let counter_clone = Rc::clone(&counter);
    runtime.go(move || {
        println!("Parent task started");

        // Spawn child tasks from within the parent task!
        for i in 0..5 {
            let counter = Rc::clone(&counter_clone);
            go(move || {
                println!("  Child task {} started", i);
                *counter.borrow_mut() += 1;
                gosched();
                println!("  Child task {} done", i);
            });
        }

        println!("Parent task done (spawned 5 children)");
    });

    println!("Running scheduler...");
    runtime.run();
    println!("All tasks completed! Counter = {}", counter.borrow());
}
