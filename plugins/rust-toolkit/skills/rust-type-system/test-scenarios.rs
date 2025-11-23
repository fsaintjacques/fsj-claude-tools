// Test scenarios for rust-type-system skill
// These represent real code that should trigger the skill's review prompts

// SCENARIO 1: Over-constrained generics
fn serialize_to_json<T: Serialize + Deserialize + Clone + Debug + PartialEq>(item: T) -> String {
    format!("{:?}", item)
}

// SCENARIO 2: Under-constrained generics
fn identity<T>(input: T) -> T {
    input
}

// SCENARIO 3: Unnecessary trait object in function argument
fn process_handler(handler: Box<dyn Fn() -> String>) {
    let result = handler();
    println!("{}", result);
}

// SCENARIO 4: Single-implementor trait (over-engineered)
trait Logger {
    fn log(&self, msg: &str);
}

struct ConsoleLogger;
impl Logger for ConsoleLogger {
    fn log(&self, msg: &str) {
        println!("{}", msg);
    }
}

// SCENARIO 5: Too many lifetime parameters
fn complex_lifetime<'a, 'b, 'c>(x: &'a str, y: &'b str) -> &'c str {
    x
}

// SCENARIO 6: Unnecessary lifetime inference
fn simple_lifetime<'a>(input: &'a str) -> &'a str {
    input
}

// SCENARIO 7: Trait bound in impl block (could be more targeted)
impl<T: Clone> std::vec::Vec<T> {
    fn duplicate_first(&self) -> Option<T> {
        self.first().map(|item| item.clone())
    }
}

// SCENARIO 8: Generic API hard to call
fn execute<T, E, F>(f: F) -> Result<T, E>
where
    F: Fn() -> Result<T, E>,
    T: Send + Sync + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    f()
}

// SCENARIO 9: Over-engineered composition
struct App {
    logger: Box<dyn Logger>,
    database: Box<dyn Database>,
    cache: Box<dyn Cache>,
}

trait Database {
    fn query(&self, sql: &str) -> String;
}

trait Cache {
    fn get(&self, key: &str) -> Option<String>;
}

// SCENARIO 10: Multiple trait objects in collection (appropriate use)
fn process_handlers(handlers: Vec<Box<dyn Fn() -> String>>) {
    for handler in handlers {
        println!("{}", handler());
    }
}
