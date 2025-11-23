---
name: rust-async-design
description: Review Rust async code for concurrency bugs, cancellation safety, resource leaks, deadlocks, and improper synchronization - identifies race conditions, lock-across-await, unbounded spawning, and error handling issues in async contexts
---

# Rust Async Design Review

## Overview

Review Rust async code for correctness, safety, and efficiency. Async is powerful but subtle - easy to introduce race conditions, deadlocks, resource leaks, and correctness bugs.

**Core principle:** Async code has more ways to fail than synchronous code. This skill identifies the most common failure modes.

**Use when:** Reviewing code with `async fn`, `tokio::spawn`, `select!`, locks, channels, or concurrent operations.

**Do NOT use this skill for:**
- Non-async concurrency (use `rust-systems-review`)
- Type system issues (use `rust-type-system`)
- Error handling philosophy (use `rust-error-handling`)

## Categories of Async Issues

### 1. Race Conditions - Data Without Proper Synchronization

**The Problem:**
Multiple tasks access shared data without synchronization. The result is unpredictable.

**Pattern: Unguarded mutation**
```rust
// ❌ Race condition
let mut count = 0;

tokio::spawn(async {
    count += 1;  // Task 1 reads, modifies, writes count
});

tokio::spawn(async {
    count += 1;  // Task 2 reads, modifies, writes count
});
// Result: count might be 1 or 2, unpredictably
```

**Questions to ask:**
- Is this data accessed by multiple tasks?
- Is it `&mut` shared (only Mutex/RwLock/Arc + Mutex/atomics are safe)?
- Could an await point cause interleaving?
- What's the worst outcome if a race occurs?

**Red flags:**
- Shared state without `Mutex`, `RwLock`, or atomics
- Comments like "should be fine" or "races shouldn't happen"
- Shared `&mut` references between tasks
- Arc<Cell<T>> (Cell is not thread-safe)
- Arc<RefCell<T>> (RefCell panics under contention)

**How to fix:**
```rust
// ✅ Synchronized access
let count = std::sync::Arc::new(std::sync::Mutex::new(0));

let count_clone = count.clone();
tokio::spawn(async move {
    *count_clone.lock().unwrap() += 1;
});

let count_clone = count.clone();
tokio::spawn(async move {
    *count_clone.lock().unwrap() += 1;
});
```

### 2. Lock Held Across Await - Holding Locks While Suspending

**The Problem:**
A lock is held while calling `await`. If another task tries to acquire the same lock, you can deadlock. If many tasks queue, throughput plummets.

**Pattern: Sync lock across await**
```rust
// ❌ Sync Mutex held across await
let data = std::sync::Mutex::new(vec![1, 2, 3]);

async fn process(data: &std::sync::Mutex<Vec<i32>>) {
    let guard = data.lock().unwrap();
    network_call(&guard).await;  // ❌ Lock held, other tasks blocked
}
```

**Why this is bad:**
1. If network_call takes 1 second, that task holds the lock for 1 second
2. All other tasks waiting for the lock are blocked for that 1 second
3. If there's a second acquire of the same lock elsewhere, deadlock

**Questions to ask:**
- Is this a sync lock (`std::sync::Mutex`)?
- Is there an await point while the guard is held?
- Could another task acquire the same lock?
- What's the longest this lock could be held?

**Red flags:**
- `std::sync::Mutex::lock()` followed by `await`
- Guard variable still in scope during `await`
- Comments saying "brief lock" (brief for code, long in async time)
- Nested locks across await

**How to fix:**
```rust
// ✅ Release lock before await
let data = Arc::new(std::sync::Mutex::new(vec![1, 2, 3]));

async fn process(data: Arc<std::sync::Mutex<Vec<i32>>>) {
    let snapshot = {
        let guard = data.lock().unwrap();
        guard.clone()  // Drop guard here
    };  // Guard released

    network_call(&snapshot).await;  // No lock held
}

// OR use async-aware locks
let data = Arc::new(tokio::sync::Mutex::new(vec![1, 2, 3]));

async fn process(data: Arc<tokio::sync::Mutex<Vec<i32>>>) {
    let guard = data.lock().await;  // This is fair to other tasks
    network_call(&*guard).await;    // Tokio mutex designed for this
}
```

### 3. Unbounded Resource Spawning - Creating Tasks Without Limits

**The Problem:**
Spawning tasks without bound causes memory exhaustion and OOM crashes.

**Pattern: Loop with unbounded spawn**
```rust
// ❌ No backpressure
async fn handle_stream() {
    for item in incoming_items {
        tokio::spawn(async move {
            process(item).await;  // Could spawn millions of tasks
        });
    }
}
```

**Questions to ask:**
- How many tasks could be spawned?
- What happens if N = 1,000,000?
- Is there backpressure (slowing producer if consumer lags)?
- Are we bounded by channel capacity, semaphore, or memory?

**Red flags:**
- `tokio::spawn` in a loop without bounds
- No channel capacity limit
- "Should be fine in practice" (won't be fine at scale)
- Processing rates assumed but not validated

**How to fix:**
```rust
// ✅ Bounded spawning with channel
let (tx, mut rx) = tokio::sync::mpsc::channel(100);  // Bounded!

// Producer
tokio::spawn(async move {
    for item in incoming {
        if tx.send(item).await.is_err() {
            break;  // Receiver closed
        }
    }
});

// Consumer
tokio::spawn(async move {
    while let Some(item) = rx.recv().await {
        process(item).await;  // Processes at capacity
    }
});
```

### 4. No Timeout on External I/O - Indefinite Waits

**The Problem:**
External services (network, databases) can hang. Without timeouts, your system hangs too.

**Pattern: No timeout**
```rust
// ❌ Could wait forever
let response = reqwest::Client::new()
    .get("https://example.com")
    .send()
    .await?;  // Network hangs, task never returns

// ❌ Database query with no timeout
let result = db.query("SELECT * FROM huge_table").execute().await?;
```

**Questions to ask:**
- Is this making an external call (network, disk, subprocess)?
- Is there a timeout?
- What's the acceptable latency?
- What happens if the timeout fires?

**Red flags:**
- `.send().await` without timeout
- `.query().await` without timeout
- "Our network is reliable" (networks fail)
- Timeout only for happy path, not errors

**How to fix:**
```rust
// ✅ Timeout added
use std::time::Duration;

let response = tokio::time::timeout(
    Duration::from_secs(5),
    reqwest::Client::new()
        .get("https://example.com")
        .send()
)
.await
.map_err(|_| "Request timeout")?
.map_err(|e| format!("Request error: {}", e))?;
```

### 5. Cancellation Unsafe - Resources Not Cleaned Up When Task Cancelled

**The Problem:**
`select!` or `timeout()` can cancel a branch. If resources aren't cleaned up, you leak.

**Pattern: Resource without cleanup**
```rust
// ❌ File might not flush if timeout fires
async fn write_data() -> Result<()> {
    let mut file = tokio::fs::File::create("data.txt").await?;

    tokio::select! {
        _ = file.write_all(b"data") => {
            file.sync_all().await?;
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => {
            // Timeout - file dropped, data not flushed
        }
    }

    Ok(())
}
```

**Questions to ask:**
- Can this task be cancelled (select!, timeout, JoinHandle::abort)?
- Are there resources that need cleanup (files, locks, connections)?
- Is cleanup guaranteed even if cancelled?
- What state would be left behind?

**Red flags:**
- `tokio::select!` without considering cancellation
- `timeout()` without cleanup
- File I/O in select without explicit sync
- Database connections not returned to pool
- Lock held after cancellation

**How to fix:**
```rust
// ✅ Explicit cleanup before cancel point
async fn write_data() -> Result<()> {
    let mut file = tokio::fs::File::create("data.txt").await?;

    let result = tokio::select! {
        res = async {
            file.write_all(b"data").await
        } => res,
        _ = tokio::time::sleep(Duration::from_secs(1)) => {
            Err(std::io::Error::other("timeout"))
        }
    };

    // Cleanup happens here, before select! unwind
    file.sync_all().await?;
    result
}

// OR use guard patterns for RAII cleanup
struct FileGuard {
    file: tokio::fs::File,
}

impl Drop for FileGuard {
    fn drop(&mut self) {
        // Cleanup happens automatically on drop
        // But be careful: can't await in drop!
    }
}
```

### 6. Blocking Operations in Async Context - Stalling the Runtime

**The Problem:**
Sync I/O or CPU-bound work in async context blocks the runtime thread, preventing other tasks from progressing.

**Pattern: Blocking call in async**
```rust
// ❌ Blocks the executor thread
async fn read_file() {
    let data = std::fs::read("file.txt").unwrap();  // Sync I/O
    process(data).await;
}

// ❌ CPU-bound work blocking
async fn compute() {
    let result = expensive_calculation();  // Blocking computation
    send_result(result).await;
}
```

**Questions to ask:**
- Is this a sync I/O call (`std::fs`, `std::io`)?
- Is this CPU-bound without yielding?
- How long could this block?
- Are other tasks starved?

**Red flags:**
- `std::fs::read`, `std::fs::write` in async fn
- `std::process::Command::output()` in async
- Long CPU loops without `.await`
- `Thread::sleep()` instead of `tokio::time::sleep()`

**How to fix:**
```rust
// ✅ Use async-aware I/O
async fn read_file() {
    let data = tokio::fs::read("file.txt").await.unwrap();
    process(data).await;
}

// ✅ CPU-bound in dedicated thread pool
async fn compute() {
    let result = tokio::task::spawn_blocking(|| {
        expensive_calculation()
    })
    .await
    .unwrap();

    send_result(result).await;
}
```

### 7. Improper Error Handling in select! - Errors Lost or Unwrapped

**The Problem:**
`select!` can mask errors. Unwrapping in a select branch can panic.

**Pattern: Unwrap in select**
```rust
// ❌ Error handling lost
tokio::select! {
    result = operation1() => {
        let data = result.unwrap();  // Panics on error!
        process(data).await;
    }
    result = operation2() => {
        let data = result.unwrap();  // Panics on error!
        process(data).await;
    }
}

// ❌ Some branches don't handle errors
tokio::select! {
    Ok(data) = operation1() => {  // Only matches Ok, Err is ignored!
        process(data).await;
    }
    _ = operation2() => {
        println!("Operation 2 completed");
    }
}
```

**Questions to ask:**
- Are all error cases handled?
- Could unwrap panic?
- Are Err results silently ignored?
- Should one error cancel other branches?

**Red flags:**
- `.unwrap()` in select! branch
- `.expect()` without thinking through failure
- Pattern matching only Ok, not Err
- No error logging or reporting

**How to fix:**
```rust
// ✅ All branches handle errors properly
tokio::select! {
    result = operation1() => {
        match result {
            Ok(data) => process(data).await,
            Err(e) => eprintln!("Operation 1 failed: {}", e),
        }
    }
    result = operation2() => {
        match result {
            Ok(data) => process(data).await,
            Err(e) => eprintln!("Operation 2 failed: {}", e),
        }
    }
}
```

### 8. Unhandled Panics in Spawned Tasks - Silent Failures

**The Problem:**
A panic in a spawned task is silently dropped unless explicitly observed with `.await` on the JoinHandle.

**Pattern: Panic not observed**
```rust
// ❌ Panic is silently dropped
tokio::spawn(async {
    panic!("Oops!");  // No one notices
});

// Task completes normally from outside perspective
```

**Questions to ask:**
- Is this task's result observed with `.await`?
- What happens if the task panics?
- Should panic crash the whole app or be handled?
- Is there logging/observability?

**Red flags:**
- `tokio::spawn()` without `.await` on JoinHandle
- Fire-and-forget tasks in critical paths
- No error context or logging

**How to fix:**
```rust
// ✅ Panic observed
let handle = tokio::spawn(async {
    panic!("Oops!");
});

match handle.await {
    Ok(_) => println!("Task completed"),
    Err(e) => {
        if e.is_panic() {
            eprintln!("Task panicked: {:?}", e);
        }
    }
}

// ✅ Or wrap in error handling
tokio::spawn(async {
    if let Err(e) = async_operation().await {
        eprintln!("Operation failed: {}", e);
    }
});
```

## The Async Review Checklist

When reviewing async code:

### Data Synchronization
- [ ] All shared data protected by `Mutex`, `RwLock`, or atomics
- [ ] No unguarded mutations across tasks
- [ ] No Arc<Cell<T>> or Arc<RefCell<T>> for thread-safe access
- [ ] Race conditions considered and ruled out

### Lock Safety
- [ ] No sync locks held across await points
- [ ] Lock scope minimized
- [ ] Async locks used when appropriate
- [ ] No nested lock acquisition (deadlock risk)

### Resource Management
- [ ] Spawned tasks are bounded (backpressure exists)
- [ ] External I/O has timeouts
- [ ] Resource cleanup happens on cancellation
- [ ] No resource leaks on error or panic

### Cancellation Safety
- [ ] Tasks cancellable without resource leaks
- [ ] Files flushed, connections closed
- [ ] Cleanup code executes on cancel
- [ ] State consistent after cancellation

### Blocking Operations
- [ ] No sync I/O in async context
- [ ] No long CPU work without yielding
- [ ] Blocking work moved to `spawn_blocking` if necessary
- [ ] Async-aware alternatives used

### Error Handling
- [ ] All error cases handled in select!
- [ ] No unwrap in error paths
- [ ] Task panics observed with `.await`
- [ ] Errors logged and propagated appropriately

### Correctness
- [ ] No race conditions
- [ ] No deadlocks
- [ ] No indefinite waits
- [ ] State transitions clear

## Common Anti-Patterns

| Pattern | Problem | Fix |
|---------|---------|-----|
| `Arc<Mutex<T>>` always | Good for sync, but causes contention | Use `RwLock` if mostly reads, or async `Mutex` |
| Unbounded `spawn()` | OOM crashes | Use bounded channels with backpressure |
| No timeout | Hangs | Add `tokio::time::timeout()` |
| `.unwrap()` in select! | Panics | Handle `Err` explicitly |
| Sync lock across await | Deadlock + contention | Drop lock before await |
| Fire-and-forget tasks | Silent failures | Observe JoinHandle with `.await` |
| `std::fs` in async | Blocks executor | Use `tokio::fs` |
| Panic in task | Silent failure | Match JoinHandle `.await` result |

## Discussion Format

When issues arise, use this format:

**Pattern identified:** "Lock acquired before await"

**Question:** "Can this lock be held while the network call completes?"

**Concern:** "If the lock is held for the duration of the network call, other tasks are blocked"

**Suggestion:** "Drop the lock before calling `network_operation().await`"

## Red Flags That Require Immediate Attention

- [ ] Race condition (unprotected shared mutation)
- [ ] Lock held across await
- [ ] No timeout on external I/O
- [ ] Unbounded task spawning
- [ ] Panic in spawned task not observed
- [ ] Sync I/O in async context
- [ ] Cancellation leaves dangling resources
- [ ] Deadlock from nested lock acquisition

## Example: Good Async Code

```rust
// ✅ Well-designed async
async fn process_stream(
    mut rx: tokio::sync::mpsc::Receiver<Item>,
) -> Result<()> {
    // Bounded channel provides backpressure

    while let Some(item) = rx.recv().await {
        // Timeout on external I/O
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            process_with_network(&item),
        ).await;

        match result {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => eprintln!("Processing error: {}", e),
            Err(_) => eprintln!("Processing timeout"),
        }
    }

    Ok(())
}

// External I/O with proper error handling
async fn process_with_network(item: &Item) -> Result<()> {
    let response = reqwest::Client::new()
        .post("https://api.example.com/process")
        .json(item)
        .send()
        .await?;

    Ok(())
}
```
