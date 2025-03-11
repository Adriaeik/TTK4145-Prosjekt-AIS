// Import necessary modules
use std::sync::Arc; // Arc is for atomic reference counting to share data safely across threads/tasks.
use tokio::sync::Mutex; // Mutex ensures exclusive access to a resource, preventing data races.
use tokio::time::{sleep, Duration}; // Sleep is used to simulate work or delays in asynchronous tasks.

// Tokio is a runtime for asynchronous programming in Rust.
// - It allows you to write asynchronous code (like tasks) that run concurrently.
// - It is **cooperative**: tasks explicitly yield control (via `.await`) to allow others to run.
// - It sits on top of the system's **preemptive** scheduling (OS-level threads) but provides fine-grained control at the task level.

// **Pros of Tokio's Cooperative Model:**
// - **Efficiency:** Less context switching compared to preemptive threads.
// - **Determinism:** Tasks yield explicitly, leading to predictable execution.
// - **Scalability:** Tasks are lightweight compared to OS threads.
//
// **Cons:**
// - **Developer Responsibility:** Tasks must yield control explicitly (`await`).
// - **Blocking Issues:** A task that never yields can block other tasks.
// - **Complexity:** Requires understanding async concepts and cooperative multitasking.

#[tokio::main] // This macro sets up the Tokio runtime for asynchronous execution.
async fn main() {
    // Create a shared resource (`number`) protected by a Mutex.
    // Arc (Atomic Reference Counting) allows safe shared ownership of the resource between tasks.
    let number = Arc::new(Mutex::new(0)); // Initial value of the shared resource is 0.

    // Clone the Arc to share the resource across multiple tasks.
    let number_clone1 = Arc::clone(&number);
    let number_clone2 = Arc::clone(&number);

    // Task 1: Increment the shared number.
    // `tokio::spawn` creates a lightweight asynchronous task that runs concurrently.
    let task1 = tokio::spawn(async move {
        for _ in 0..5 {
            // Lock the Mutex to gain exclusive access to the resource.
            let mut num = number_clone1.lock().await; // `.await` yields control while waiting for the lock.
            *num += 1; // Increment the number.
            println!("Incremented: {}", *num); // Print the new value.
            sleep(Duration::from_millis(500)).await; // Simulate work and yield control.
        } // Mutex is automatically unlocked here because `num` goes out of scope.
    });

    // Task 2: Decrement the shared number.
    let task2 = tokio::spawn(async move {
        for _ in 0..5 {
            // Lock the Mutex to gain exclusive access to the resource.
            let mut num = number_clone2.lock().await; // `.await` yields control while waiting for the lock.
            *num -= 1; // Decrement the number.
            println!("Decremented: {}", *num); // Print the new value.
            sleep(Duration::from_millis(300)).await; // Simulate work and yield control.
        } // Mutex is automatically unlocked here because `num` goes out of scope.
    });

    // `tokio::join!` waits for both tasks to finish concurrently.
    let _ = tokio::join!(task1, task2);

    // Access the final value of the number.
    let final_value = number.lock().await; // Lock the Mutex again to read the final value.
    println!("Final value: {}", *final_value); // Print the final value.
                                               // Mutex is automatically unlocked here because `final_value` goes out of scope.
}
