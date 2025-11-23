// Test scenarios for rust-error-handling skill
// Common error handling anti-patterns and issues

// SCENARIO 1: Context loss - original error disappears
fn parse_config(input: &str) -> Result<Config, String> {
    let parsed = serde_json::from_str::<Value>(input)
        .map_err(|e| format!("Error: {}", e))?;  // ❌ Config parsing context lost

    extract_config(parsed)
}

fn extract_config(value: Value) -> Result<Config, String> {
    value.as_object()
        .ok_or_else(|| "Invalid type".to_string())  // ❌ What was invalid? No context
}

// SCENARIO 2: Overly generic error type
fn load_user(id: u32) -> Result<User, String> {  // ❌ String loses all error info
    db.query("SELECT * FROM users WHERE id = ?", id)
        .map_err(|e| e.to_string())?
}

// SCENARIO 3: Error recovery not considered
fn fetch_data(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let response = reqwest::blocking::get(url)?;  // ❌ Network error, should retry?
    Ok(response.bytes()?.to_vec())
}

// SCENARIO 4: Silent errors (no logging, no context)
async fn process_items(items: Vec<Item>) {
    for item in items {
        if let Err(_) = process_item(&item).await {
            // ❌ Error silently dropped, no logging, no context
        }
    }
}

async fn process_item(_item: &Item) -> Result<(), ProcessError> {
    Ok(())
}

// SCENARIO 5: Catching and re-throwing without adding context
fn orchestrate() -> Result<(), MyError> {
    match step1() {
        Ok(v) => step2(v),
        Err(e) => Err(e),  // ❌ Just re-throws, doesn't say where it failed
    }
}

fn step1() -> Result<i32, MyError> { Ok(1) }
fn step2(_: i32) -> Result<(), MyError> { Ok(()) }

// SCENARIO 6: Using wrong error type for context
fn validate_email(email: &str) -> Result<(), String> {
    if !email.contains('@') {
        return Err("Invalid email".to_string());  // ❌ Loses which field, what was expected
    }
    Ok(())
}

// SCENARIO 7: Not distinguishing recoverable vs fatal errors
fn try_operation() -> Result<(), std::io::Error> {
    std::fs::read("file.txt")?  // ❌ Not found is recoverable, permission denied might be fatal
}

// SCENARIO 8: Unwrap in production code
fn get_config() -> Config {
    serde_json::from_str(include_str!("config.json")).unwrap()  // ❌ Panics if config malformed
}

// SCENARIO 9: Error doesn't implement Display
#[derive(Debug)]
struct CustomError(String);
// ❌ Missing impl Display, can't use with ? operator in many contexts

// SCENARIO 10: Losing original error in conversion
fn convert_db_error(e: DbError) -> ApiError {
    ApiError {
        message: "Database error".to_string(),  // ❌ Original error lost
    }
}

// SCENARIO 11: Good error with context
#[derive(Debug)]
enum ParseError {
    InvalidFormat { field: String, value: String },
    MissingField { field: String },
    DatabaseError { source: DbError },
}

fn parse_config_good(input: &str) -> Result<Config, ParseError> {
    let parsed = serde_json::from_str::<Value>(input)
        .map_err(|e| ParseError::DatabaseError { source: DbError::from(e) })?;

    extract_config_good(parsed)
}

fn extract_config_good(value: Value) -> Result<Config, ParseError> {
    value.as_object()
        .ok_or_else(|| ParseError::InvalidFormat {
            field: "root".to_string(),
            value: value.to_string(),
        })
}

#[derive(Debug)]
struct DbError;
impl From<serde_json::Error> for DbError {
    fn from(_: serde_json::Error) -> Self { DbError }
}

// SCENARIO 12: Recoverable error with retry
async fn fetch_with_retry(url: &str, max_retries: usize) -> Result<Vec<u8>, FetchError> {
    for attempt in 0..max_retries {
        match fetch_once(url).await {
            Ok(data) => return Ok(data),
            Err(FetchError::Network(_)) if attempt < max_retries - 1 => {
                // ✅ Distinguishes network (recoverable) from other errors
                tokio::time::sleep(tokio::time::Duration::from_millis(100 * 2_u64.pow(attempt as u32))).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Err(FetchError::Network("Max retries exceeded".into()))
}

#[derive(Debug)]
enum FetchError {
    Network(String),
    Parse(String),
    Timeout,
}

async fn fetch_once(_url: &str) -> Result<Vec<u8>, FetchError> {
    Ok(vec![])
}

// SCENARIO 13: Error type that derives Error trait
use std::error::Error;
use std::fmt;

#[derive(Debug)]
enum FileError {
    NotFound(String),
    PermissionDenied(String),
    IoError(std::io::Error),
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::NotFound(path) => write!(f, "File not found: {}", path),
            FileError::PermissionDenied(path) => write!(f, "Permission denied: {}", path),
            FileError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl Error for FileError {}

// SCENARIO 14: Using thiserror crate for less boilerplate
// #[derive(thiserror::Error, Debug)]
// enum ApiError {
//     #[error("Validation failed for field {field}: {reason}")]
//     Validation { field: String, reason: String },
//
//     #[error("Database error")]
//     Database(#[from] DbError),
//
//     #[error("Network timeout")]
//     Timeout,
// }

// SCENARIO 15: Error propagation with context
fn validate_user_input(input: &str) -> Result<User, ValidationError> {
    let parsed = parse_json(input)
        .map_err(|e| ValidationError::ParseError {
            field: "input".to_string(),
            source: e,
        })?;

    extract_user(parsed)
        .map_err(|e| ValidationError::ExtractionError {
            source: e,
        })
}

#[derive(Debug)]
enum ValidationError {
    ParseError { field: String, source: ParseError },
    ExtractionError { source: ParseError },
}

#[derive(Debug)]
enum ParseError {
    Invalid,
}

fn parse_json(_input: &str) -> Result<Value, ParseError> {
    Ok(Value::Null)
}

fn extract_user(_value: Value) -> Result<User, ParseError> {
    Ok(User { id: 1 })
}

struct User { id: i32 }
enum Value { Null }

// SCENARIO 16: Error not properly documented
fn risky_operation() -> Result<Data, RiskyError> {
    // What errors can this return?
    // When would each occur?
    // What should caller do?
    Ok(Data)
}

#[derive(Debug)]
enum RiskyError {
    Unknown,
}

struct Data;

// SCENARIO 17: Good - error types with documentation
/// Error type for configuration loading operations.
///
/// # Variants
///
/// - `NotFound` - Configuration file does not exist (recoverable, use defaults)
/// - `ParseError` - Configuration is malformed (fatal, human intervention needed)
/// - `Permission` - No read permission on config file (fatal for this process)
#[derive(Debug)]
enum ConfigError {
    NotFound(String),
    ParseError { line: usize, message: String },
    Permission(String),
}

impl ConfigError {
    /// Returns true if this error is recoverable (can use defaults)
    pub fn is_recoverable(&self) -> bool {
        matches!(self, ConfigError::NotFound(_))
    }
}
