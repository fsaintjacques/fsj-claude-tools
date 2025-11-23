// Test scenarios for rust-async-design skill
// These represent real async code that should trigger specific review questions

// SCENARIO 1: Race condition - reading state without proper synchronization
async fn process_with_race_condition() {
    let mut count = 0;

    let task1 = tokio::spawn(async {
        count += 1;  // ❌ Data race - no synchronization
    });

    let task2 = tokio::spawn(async {
        count += 1;  // ❌ Data race
    });

    task1.await.ok();
    task2.await.ok();
}

// SCENARIO 2: Lock held across await point
async fn lock_across_await() {
    let data = std::sync::Mutex::new(vec![1, 2, 3]);

    let guard = data.lock().unwrap();  // ❌ Sync lock held...
    process_async(&guard).await;       // ❌ ...across await point
}

async fn process_async(_data: &[i32]) {}

// SCENARIO 3: Unbounded resource spawning
async fn spawn_unbounded() {
    for i in 0..100_000 {
        tokio::spawn(async move {
            println!("Task {}", i);  // ❌ No backpressure, unbounded spawning
        });
    }
}

// SCENARIO 4: No timeout on external I/O
async fn no_timeout() {
    let _result = reqwest::Client::new()
        .get("https://example.com")
        .send()
        .await;  // ❌ No timeout - could hang forever
}

// SCENARIO 5: Cancellation unsafe - resource not cleaned up
async fn cancellation_unsafe() {
    let file = tokio::fs::File::create("/tmp/data").await.ok();

    tokio::select! {
        _ = some_operation() => {
            // File drops here if task cancelled - may not flush
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
            // Task cancelled
        }
    }
}

async fn some_operation() {}

// SCENARIO 6: Blocking call in async context
async fn blocking_in_async() {
    let data = std::fs::read("file.txt");  // ❌ Blocking I/O in async fn
    println!("{:?}", data);
}

// SCENARIO 7: Improper error handling with select!
async fn select_error_handling() {
    tokio::select! {
        result = async_operation1() => {
            // What if result is Err? Not handled
            let data = result.unwrap();
            process(data).await;
        }
        result = async_operation2() => {
            // Same issue here
            let data = result.unwrap();
            process(data).await;
        }
    }
}

async fn async_operation1() -> Result<String, String> {
    Ok("data".to_string())
}

async fn async_operation2() -> Result<String, String> {
    Ok("data".to_string())
}

async fn process(_: String) {}

// SCENARIO 8: Unhandled panics in spawned tasks
async fn unhandled_panic() {
    tokio::spawn(async {
        panic!("Task panicked");  // ❌ Panic silently dropped, not observed
    });

    // Main task continues, panic is lost
}

// SCENARIO 9: Excessive cloning for Arc<T>
async fn excessive_cloning() {
    let data = std::sync::Arc::new(vec![1, 2, 3, 4, 5]);

    for i in 0..1000 {
        let data_clone = data.clone();  // ❌ Cloning Arc excessively in loop
        tokio::spawn(async move {
            println!("{}: {:?}", i, data_clone);
        });
    }
}

// SCENARIO 10: Deadlock with multiple locks
async fn potential_deadlock() {
    let lock1 = tokio::sync::Mutex::new(1);
    let lock2 = tokio::sync::Mutex::new(2);

    let task1 = tokio::spawn(async {
        let g1 = lock1.lock().await;  // Task 1: acquires lock1
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let _g2 = lock2.lock().await;  // Task 1: waits for lock2
    });

    let task2 = tokio::spawn(async {
        let _g2 = lock2.lock().await;  // Task 2: acquires lock2
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let _g1 = lock1.lock().await;  // Task 2: waits for lock1
    });

    task1.await.ok();
    task2.await.ok();
}

// SCENARIO 11: Good async pattern - cancellation safe
async fn cancellation_safe() {
    let mut file = tokio::fs::File::create("/tmp/data").await.ok();

    let result = tokio::select! {
        res = async {
            // Work that can be safely cancelled
            some_computation().await
        } => res,
        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
            // Timeout - file already dropped if spawned, or properly
            // cleaned up as task scope ends
            Err("timeout")
        }
    };

    if let Some(f) = file.as_mut() {
        f.sync_all().await.ok();  // Explicit cleanup
    }
}

async fn some_computation() -> Result<String, &'static str> {
    Ok("result".to_string())
}

// SCENARIO 12: Proper backpressure handling
async fn backpressure_handled() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);  // Bounded channel

    tokio::spawn(async move {
        for i in 0..1000 {
            if tx.send(i).await.is_err() {
                // Receiver dropped, can't continue
                break;
            }
        }
    });

    while let Some(value) = rx.recv().await {
        process_item(value).await;
    }
}

async fn process_item(_: i32) {}
