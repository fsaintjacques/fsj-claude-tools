---
name: rust-error-handling
description: Review Rust code for error handling strategy, context preservation, and recovery - identifies context loss, unrecoverable error mixing, undocumented error types, and missing error propagation patterns
---

# Rust Error Handling Review

## Overview

Review Rust error handling for correctness, context preservation, and debuggability. Poor error handling causes silent failures, lost context, and hours of debugging.

**Core principle:** An error without context is useless. Every error should carry enough information for debugging.

**Use when:** Reviewing error types, error handling patterns, Result usage, or error propagation strategy.

**Do NOT use this skill for:**
- Type system design (use `rust-type-system`)
- Async error handling specifics (use `rust-async-design`)
- Unsafe code (use `rust-systems-review`)

## Categories of Error Handling Issues

### 1. Context Loss - Original Error Disappears

**The Problem:**
Converting errors without preserving information makes debugging impossible.

**Pattern: Erasing error details**
```rust
// ❌ Context lost
fn parse_config(input: &str) -> Result<Config, String> {
    let value = serde_json::from_str(input)
        .map_err(|e| format!("Error: {}", e))?;  // Original error details lost

    extract_config(value)
}

// ❌ What was invalid? Where?
fn extract_config(value: Value) -> Result<Config, String> {
    value.as_object()
        .ok_or_else(|| "Invalid type".to_string())?  // No context about what failed
}
```

**Questions to ask:**
- What information does the original error contain?
- Is that information accessible to the caller?
- Could the caller debug the issue with just this error message?
- Is there a field, line number, or context missing?

**Red flags:**
- `.map_err(|e| "Error")` (all errors become generic)
- `.map_err(|e| e.to_string())` (type information lost)
- `.map_err(|_| MyError)` (original error ignored)
- Errors converted without preserving source
- No information about which operation failed

**How to fix: Preserve context**
```rust
// ✅ Context preserved
#[derive(Debug)]
enum ConfigError {
    ParseError { source: serde_json::Error },
    InvalidType { expected: &'static str, actual: &'static str },
    MissingField { field: String },
}

fn parse_config(input: &str) -> Result<Config, ConfigError> {
    let value = serde_json::from_str(input)
        .map_err(|e| ConfigError::ParseError { source: e })?;

    extract_config(value)
}

fn extract_config(value: Value) -> Result<Config, ConfigError> {
    value.as_object()
        .ok_or_else(|| ConfigError::InvalidType {
            expected: "object",
            actual: value.type_name(),
        })
}

// ✅ Using thiserror for less boilerplate
use thiserror::Error;

#[derive(Error, Debug)]
enum ConfigError {
    #[error("Failed to parse JSON")]
    ParseError(#[from] serde_json::Error),

    #[error("Expected object, got {actual}")]
    InvalidType { actual: String },

    #[error("Missing field: {field}")]
    MissingField { field: String },
}
```

### 2. Overly Generic Error Types - All Errors Look the Same

**The Problem:**
Using `String` or generic `Box<dyn Error>` makes it impossible to distinguish error types.

**Pattern: String error type**
```rust
// ❌ String loses all structure
fn fetch_user(id: u32) -> Result<User, String> {
    // Is this a network error? Database error? Parsing error?
    // Caller has no way to know
    db.query("SELECT * FROM users WHERE id = ?", id)
        .map_err(|e| e.to_string())?
}

// ❌ Too generic
fn operation() -> Result<T, Box<dyn std::error::Error>> {
    // Caller can't distinguish recoverable from fatal
    // Caller can't implement specific retry logic
    // Caller has to pattern match on error message
}
```

**Questions to ask:**
- Could the caller handle different error types differently?
- Is there a pattern in the error messages?
- Does the caller need to retry, log, or propagate?
- Could the error type be split?

**Red flags:**
- `Result<T, String>` (no structure)
- `Result<T, Box<dyn Error>>` without documentation
- Error messages that code pattern-matches on
- Multiple error sources with no distinction
- Caller using `e.to_string()` to inspect error type

**How to fix: Structured error types**
```rust
// ✅ Specific error types
#[derive(Error, Debug)]
enum UserError {
    #[error("User not found: {id}")]
    NotFound { id: u32 },

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Permission denied for user {id}")]
    PermissionDenied { id: u32 },
}

fn fetch_user(id: u32) -> Result<User, UserError> {
    db.query("SELECT * FROM users WHERE id = ?", id)
        .await
        .map_err(UserError::Database)
        .and_then(|rows| {
            if rows.is_empty() {
                Err(UserError::NotFound { id })
            } else {
                Ok(rows[0].clone())
            }
        })
}

// Caller can now handle specific errors
match fetch_user(id) {
    Ok(user) => process(user),
    Err(UserError::NotFound { id }) => log_not_found(id),
    Err(UserError::Database(e)) => log_db_error(e),
    Err(UserError::PermissionDenied { id }) => deny_access(id),
}
```

### 3. Undocumented Error Types - What Can Go Wrong?

**The Problem:**
Function returns an error type, but doesn't document what errors it can produce.

**Pattern: Undocumented errors**
```rust
// ❌ What errors can this return?
fn process_data(data: &str) -> Result<Output, ProcessError> {
    // Is ProcessError::NetworkError possible?
    // Is ProcessError::Timeout possible?
    // What should caller do?
    todo!()
}

// ❌ Public error type with no docs
#[derive(Debug)]
pub enum ApiError {
    Unknown,
}
```

**Questions to ask:**
- Which error variants can actually occur?
- Under what conditions?
- Is this an incomplete error type?
- Should caller retry, log, or propagate?
- Is the error recoverable?

**Red flags:**
- Error type with no documentation
- Variant named `Unknown` or `Other`
- No explanation of when each error occurs
- No guidance on caller's responsibility
- Public error type with unclear semantics

**How to fix: Document error types**
```rust
// ✅ Well-documented error type
/// Errors that can occur during data processing.
///
/// # Recoverable Errors
///
/// - `Timeout` - Processing took too long. Caller should retry with backoff.
/// - `TemporaryFailure` - Transient network issue. Caller should retry.
///
/// # Fatal Errors
///
/// - `ValidationFailed` - Input data was invalid. Human intervention needed.
/// - `Unsupported` - Feature not supported in current configuration.
///
/// # When to Retry
///
/// Retry on `Timeout` and `TemporaryFailure`. Do not retry on validation or unsupported.
#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("Processing timeout after {duration:?}")]
    Timeout { duration: Duration },

    #[error("Temporary failure: {reason}")]
    TemporaryFailure { reason: String },

    #[error("Validation failed: {message}")]
    ValidationFailed { message: String },

    #[error("Feature not supported: {feature}")]
    Unsupported { feature: String },
}

impl ProcessError {
    /// Returns true if the error is retriable (temporary)
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            ProcessError::Timeout { .. } | ProcessError::TemporaryFailure { .. }
        )
    }
}

/// Process data with documented error handling.
///
/// # Errors
///
/// Returns:
/// - `ProcessError::ValidationFailed` if input is invalid (non-retriable)
/// - `ProcessError::Timeout` if processing exceeds time limit (retriable)
/// - `ProcessError::TemporaryFailure` if transient network issue (retriable)
pub fn process_data(data: &str) -> Result<Output, ProcessError> {
    // ...
    todo!()
}
```

### 4. Not Distinguishing Recoverable vs Fatal Errors

**The Problem:**
All errors treated the same, but some can be retried while others are permanent.

**Pattern: No recovery strategy**
```rust
// ❌ No distinction between error types
fn load_data(path: &str) -> Result<Data, std::io::Error> {
    // NotFound might be recoverable (use defaults)
    // PermissionDenied is fatal (can't fix programmatically)
    // But both return same error type
    std::fs::read(path)
}

// Caller has no way to distinguish
match load_data("config.json") {
    Ok(data) => process(data),
    Err(e) => {
        // Retry? Log and exit? Use defaults?
        // Error doesn't say
    }
}
```

**Questions to ask:**
- Which errors can the caller recover from?
- Which errors are fatal?
- Should some be retried?
- Should some use defaults?
- Does error type communicate intent?

**Red flags:**
- All IO errors treated the same
- No attempt at recovery
- Comments like "this will never happen"
- Blanket `.map_err(|_| Error)` for multiple error sources
- Error type doesn't distinguish by severity

**How to fix: Distinguish recoverable errors**
```rust
// ✅ Structured errors with recovery info
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {path}")]
    NotFound { path: String },

    #[error("Config file {path} is not readable: {reason}")]
    PermissionDenied { path: String, reason: String },

    #[error("Failed to parse config: {0}")]
    ParseError(#[from] serde_json::Error),
}

impl ConfigError {
    pub fn is_recoverable(&self) -> bool {
        matches!(self, ConfigError::NotFound { .. })
    }
}

pub fn load_config(path: &str) -> Result<Config, ConfigError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => ConfigError::NotFound {
                path: path.to_string(),
            },
            std::io::ErrorKind::PermissionDenied => ConfigError::PermissionDenied {
                path: path.to_string(),
                reason: e.to_string(),
            },
            _ => return Err(e),
        })?;

    serde_json::from_str(&contents)
        .map_err(ConfigError::ParseError)
}

// Caller can now handle intelligently
match load_config("config.json") {
    Ok(config) => process(config),
    Err(e) if e.is_recoverable() => use_defaults(),
    Err(e) => {
        eprintln!("Fatal error: {}", e);
        std::process::exit(1);
    }
}
```

### 5. Losing Source Error in Conversion

**The Problem:**
Converting errors by wrapping them without preserving the original.

**Pattern: Source error lost**
```rust
// ❌ Original error lost
fn api_call() -> Result<Response, ApiError> {
    let response = reqwest::blocking::get("https://api.example.com")
        .map_err(|_| ApiError::NetworkError)?;  // reqwest error discarded

    Ok(response)
}

// ❌ Chain broken
fn database_operation() -> Result<User, CustomError> {
    sqlx::query_as("SELECT * FROM users")
        .fetch_one(&db)
        .await
        .map_err(|e| CustomError {
            message: "Database error".to_string(),
            // Original sqlx error lost
        })
}
```

**Questions to ask:**
- Is the original error preserved?
- Can the caller inspect the root cause?
- Does the error type implement `Error` trait?
- Is the source field set correctly?

**Red flags:**
- `.map_err(|_| NewError)` (original discarded)
- Error type with no `source()` field
- No `#[from]` attribute in `thiserror`
- Chained errors lose intermediate context

**How to fix: Preserve error chain**
```rust
// ✅ Using thiserror to preserve source
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Network error")]
    Network(#[from] reqwest::Error),

    #[error("Failed to parse response")]
    ParseError(#[from] serde_json::Error),
}

fn api_call() -> Result<Response, ApiError> {
    let response = reqwest::blocking::get("https://api.example.com")?;
    // Error automatically wrapped, source preserved
    Ok(response)
}

// ✅ Or manual implementation
#[derive(Debug)]
pub enum DatabaseError {
    Query(sqlx::Error),
    Connection(String),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::Query(e) => write!(f, "Query error: {}", e),
            DatabaseError::Connection(s) => write!(f, "Connection error: {}", s),
        }
    }
}

impl std::error::Error for DatabaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DatabaseError::Query(e) => Some(e),
            DatabaseError::Connection(_) => None,
        }
    }
}

fn database_operation() -> Result<User, DatabaseError> {
    sqlx::query_as("SELECT * FROM users")
        .fetch_one(&db)
        .await
        .map_err(DatabaseError::Query)  // Source preserved
}
```

### 6. Silent Errors - Errors Ignored or Lost

**The Problem:**
Errors occur but are never logged, reported, or acted upon.

**Pattern: Silent error handling**
```rust
// ❌ Error silently dropped
async fn process_items(items: Vec<Item>) {
    for item in items {
        if let Err(_) = process_item(&item).await {
            // What error occurred? Where?
            // Should we stop? Retry? Log?
        }
    }
}

// ❌ Fire-and-forget with no error handling
tokio::spawn(async {
    if let Err(e) = operation().await {
        // Error lost, caller has no idea
    }
});
```

**Questions to ask:**
- Is the error being logged?
- Is there any context attached?
- Does the caller know an error occurred?
- Should the program stop or continue?
- Is this a bug in the error handling?

**Red flags:**
- `if let Err(_) = ...` with no body
- `_ =` to silence errors
- Errors in spawned tasks not observed
- No logging or metrics for errors
- Silent degradation without error

**How to fix: Explicit error handling**
```rust
// ✅ Explicit error handling with logging
async fn process_items(items: Vec<Item>) {
    for item in items {
        match process_item(&item).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to process item {:?}: {}", item.id, e);
                // Decide: stop, skip, retry, or continue?
                // This decision should be documented
            }
        }
    }
}

// ✅ Spawned task with error observation
tokio::spawn(async {
    match operation().await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Background operation failed: {}", e);
            // Metrics, logging, etc.
        }
    }
});

// ✅ Document error handling strategy
/// Processes items, skipping items with errors.
///
/// Errors are logged but do not stop processing.
/// Non-fatal errors should be retried elsewhere.
async fn process_items_skip_on_error(items: Vec<Item>) {
    for item in items {
        match process_item(&item).await {
            Ok(_) => {}
            Err(e) => {
                log::error!("Processing item {}: {}", item.id, e);
                // Continue processing other items
            }
        }
    }
}
```

### 7. Catching and Re-throwing Without Context

**The Problem:**
Errors propagated without adding information about where they occurred.

**Pattern: Re-throw without context**
```rust
// ❌ Error propagated with no added context
fn orchestrate() -> Result<Data, Error> {
    match step1() {
        Ok(v) => step2(v),
        Err(e) => Err(e),  // Just passes through - where did it fail?
    }
}

// ❌ Multiple sources, same error type, no context
fn load_config() -> Result<Config, std::io::Error> {
    let file = std::fs::read_to_string("config.json")?;
    // Is this a parsing error or file read error? Error doesn't say
    let config = serde_json::from_str(&file)?;
    Ok(config)
}
```

**Questions to ask:**
- Does the caller know which operation failed?
- Could the error be from different sources?
- Is any context added about the failure?
- Should this be a different error type?

**Red flags:**
- `match ... { Ok(v) => ..., Err(e) => Err(e) }`
- Multiple IO or parse operations with same error type
- Error message identical to original
- No indication of which step failed

**How to fix: Add context while propagating**
```rust
// ✅ Context added while propagating
fn orchestrate() -> Result<Data, OrchestrateError> {
    let v = step1()
        .map_err(|e| OrchestrateError::Step1Failed(e))?;

    step2(v)
        .map_err(|e| OrchestrateError::Step2Failed(e))
}

#[derive(Error, Debug)]
enum OrchestrateError {
    #[error("Step 1 failed: {0}")]
    Step1Failed(String),

    #[error("Step 2 failed: {0}")]
    Step2Failed(String),
}

// ✅ Distinguish sources using custom error type
#[derive(Error, Debug)]
pub enum LoadConfigError {
    #[error("Failed to read config file")]
    IoError(#[from] std::io::Error),

    #[error("Config file is malformed")]
    ParseError(#[from] serde_json::Error),
}

fn load_config() -> Result<Config, LoadConfigError> {
    let file = std::fs::read_to_string("config.json")?;
    let config = serde_json::from_str(&file)?;
    Ok(config)
}
```

## Error Handling Checklist

When reviewing error handling:

### Error Types
- [ ] Error types are specific, not generic (String, Box<dyn Error>)
- [ ] Variants are documented with explanation
- [ ] Variants distinguish recoverable from fatal
- [ ] Error type implements `Display` and `Error` traits
- [ ] `source()` preserves error chain

### Context Preservation
- [ ] Original errors are wrapped, not discarded
- [ ] Context added (field names, values, operation)
- [ ] Error messages are descriptive and actionable
- [ ] Stack trace information preserved where needed
- [ ] No `map_err(|_| ...)` that loses source

### Error Propagation
- [ ] `?` operator used appropriately
- [ ] Error context added while propagating
- [ ] Different error sources distinguished
- [ ] Error chain traceable from top to bottom
- [ ] No re-wrapping at multiple levels

### Error Recovery
- [ ] Recoverable errors documented
- [ ] Retry logic for transient errors
- [ ] Fallback values for expected failures
- [ ] Logging for all significant errors
- [ ] No silent error swallowing

### Documentation
- [ ] Function docs explain what errors it can return
- [ ] Variants documented (when/why they occur)
- [ ] Caller's responsibility explained
- [ ] Recovery strategy suggested
- [ ] No undocumented error types

### Testing
- [ ] Error cases tested, not just happy path
- [ ] Error messages verified
- [ ] Error recovery tested
- [ ] Panic cases covered
- [ ] Logging verified

## Red Flags Requiring Immediate Attention

- [ ] Error type is just `String`
- [ ] `map_err(|_| ...)` (source discarded)
- [ ] Error variant named `Unknown` or `Other`
- [ ] Undocumented error type in public API
- [ ] `unwrap()` in production code
- [ ] Silent error swallowing (`if let Err(_)` with no body)
- [ ] Multiple error sources with no distinction
- [ ] No logging for errors
- [ ] Spawned task panics not observed
- [ ] Errors with no Display implementation

## Choosing an Error Handling Approach

When reviewing error handling, you'll encounter three strategies. Each is appropriate for different contexts.

### Strategy 1: `thiserror` Crate - Structured Errors

**Use `thiserror` when:**
- Code is a **library** with public error types
- Error variants are **specific and distinct** (not all errors are the same)
- Callers **need to match on specific errors** (different handling for different failures)
- Error **source chain matters** (callers want to inspect root cause)
- Code already uses `thiserror` (consistency)

**Example: Library code**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid JSON at line {line}")]
    JsonError { line: usize, #[from] source: serde_json::Error },

    #[error("Missing required field: {field}")]
    MissingField { field: String },
}

// Callers can distinguish:
match parse(input) {
    Err(ParseError::MissingField { field }) => use_default_for(field),
    Err(ParseError::JsonError { .. }) => bail!("Config malformed"),
}
```

**Red flag if code is NOT using thiserror but should:**
- Public API returns custom error type
- Error variants exist but aren't `#[derive(Error)]`
- Errors written manually with `impl Error` trait
- Multiple error variants but caller can't distinguish them

**When to suggest thiserror:**
```
Code pattern: Custom enum with manual Display + Error impl
Size: 20+ lines of boilerplate
Suggestion: "Use #[derive(Error)] from thiserror to reduce 20 lines to 5"
```

### Strategy 2: `anyhow` Crate - Dynamic Errors

**Use `anyhow` when:**
- Code is **internal/application logic** (not a library)
- Error types are **unknown ahead of time** (multiple external sources)
- Callers **don't need to pattern match** (just need good error messages)
- You want **ergonomic error chains** without defining types
- Code already uses `anyhow` (consistency)

**Example: Application code**
```rust
use anyhow::{Result, Context};

// No custom error type needed
fn load_config(path: &str) -> Result<Config> {
    let contents = std::fs::read_to_string(path)
        .context("Failed to read config file")?;

    let config = serde_json::from_str(&contents)
        .context("Failed to parse config")?;

    Ok(config)
}

// Caller gets full error chain automatically:
// Error: Failed to parse config
// Caused by:
//     0: expected value at line 1 column 5
```

**Red flag if code is NOT using anyhow but should:**
- Application code with many error sources
- Manual error chaining with `map_err(|e| CustomError(format!(...)))`
- Caller code that just logs/reports errors (not pattern matching)
- Error messages doing all the heavy lifting

**When to suggest anyhow:**
```
Code pattern: Many .map_err() calls, string formatting
Problem: Error chain unclear, context scattered
Suggestion: "Use anyhow::Result<T> with .context() for cleaner chains"
```

### Strategy 3: Manual Implementation - Custom Control

**Use manual `impl Error` when:**
- You **can't use dependencies** (no_std, minimal env)
- Error type is **extremely simple** (1-2 variants)
- You need **complete control** over display/source behavior
- Code already uses manual impl (consistency)

**Example: Minimal/no_std code**
```rust
#[derive(Debug)]
pub enum SimpleError {
    NotFound,
    Timeout,
}

impl std::fmt::Display for SimpleError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SimpleError::NotFound => write!(f, "Not found"),
            SimpleError::Timeout => write!(f, "Timeout"),
        }
    }
}

impl std::error::Error for SimpleError {}
```

**Red flag if code is NOT using manual impl but should:**
- no_std environment but using `thiserror` (takes `std`)
- Minimal error variants (1-3) but using full `thiserror`
- Custom error source behavior that `thiserror` doesn't support

**When to suggest manual impl:**
```
Code pattern: no_std with dependencies
Problem: thiserror not available
Suggestion: "Manual impl Error is lightweight for no_std"
```

### Decision Tree for Reviewing

```
Is this a library or application code?

  Library?
  ├─ Callers need to match on errors?
  │  ├─ Yes → Use thiserror
  │  └─ No → Use anyhow (if deps ok) or manual impl
  └─ Yes → Use thiserror (unless error is trivial)

  Application?
  ├─ Many distinct error sources?
  │  ├─ Yes → Use anyhow or thiserror
  │  └─ No → Use anyhow or manual impl
  └─ Does caller pattern match?
     ├─ Yes → Use thiserror
     └─ No → Use anyhow

  Constraints?
  ├─ no_std required? → Manual impl (thiserror won't work)
  ├─ Already using thiserror? → Continue with thiserror
  ├─ Already using anyhow? → Continue with anyhow
  └─ Nothing yet? → Recommend above based on code type
```

### Consistency Within Codebase

**Critical rule:** A single codebase should not mix strategies unnecessarily.

**Red flags for mixed approaches:**
```rust
// ❌ Same codebase mixing strategies
mod config {
    use thiserror::Error;  // Using thiserror
    #[derive(Error, Debug)]
    pub enum ConfigError { ... }
}

mod database {
    use anyhow::Result;  // Using anyhow
    pub fn query() -> Result<Data> { ... }
}

mod network {
    // Using manual impl
    pub struct NetworkError;
    impl std::error::Error for NetworkError {}
}
```

**Better: Choose one strategy for the codebase**
```rust
// ✅ Consistent approach
// If library: all code uses thiserror
// If application: all code uses anyhow
// If minimal: all code uses manual impl
```

**When to suggest consistency:**
```
Review observation: "Code uses both thiserror and anyhow"
Suggestion: "Recommend standardizing on one. For library code: thiserror.
For application: anyhow. This simplifies caller code and error handling."
```

## Common Error Handling Patterns

### Pattern: Custom Error Type with thiserror
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("Invalid input: {message}")]
    Invalid { message: String },
}
```

### Pattern: Recoverable vs Non-Recoverable
```rust
impl MyError {
    pub fn is_recoverable(&self) -> bool {
        matches!(self, MyError::Io(e) if e.kind() == std::io::ErrorKind::TimedOut)
    }
}
```

### Pattern: Error Conversion
```rust
impl From<String> for MyError {
    fn from(s: String) -> Self {
        MyError::Invalid { message: s }
    }
}
```

## Example: Well-Reviewed Error Handling

```rust
/// Errors that can occur while loading configuration.
///
/// # Recovery Guide
///
/// - `FileNotFound` - Use defaults, document that defaults were used
/// - `ParseError` - Configuration is malformed, human intervention needed
/// - `InvalidConfig` - Configuration is valid JSON but semantically wrong
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {path}")]
    FileNotFound { path: String },

    #[error("Failed to parse config file {path}: {source}")]
    ParseError {
        path: String,
        #[from]
        source: serde_json::Error,
    },

    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },
}

impl ConfigError {
    /// Returns true if this error can be recovered from
    pub fn is_recoverable(&self) -> bool {
        matches!(self, ConfigError::FileNotFound { .. })
    }
}

/// Load configuration from a file, with fallback to defaults.
///
/// # Errors
///
/// Returns `ConfigError::ParseError` if file exists but is malformed.
/// Returns `ConfigError::InvalidConfig` if configuration is semantically invalid.
pub fn load_config(path: &str) -> Result<Config, ConfigError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => ConfigError::FileNotFound { path: path.to_string() },
            _ => panic!("Unexpected IO error: {}", e),
        })?;

    let config = serde_json::from_str::<Config>(&contents)
        .map_err(|e| ConfigError::ParseError {
            path: path.to_string(),
            source: e,
        })?;

    config.validate()
        .map_err(|e| ConfigError::InvalidConfig { message: e })
}
```
