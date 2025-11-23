// Test scenarios for rust-code-review-flow skill
// Real code snippets where reviewer must determine which skills to use

// SCENARIO 1: Async code with potential race condition
async fn fetch_and_cache(url: &str) -> Result<Vec<u8>> {
    let mut cache = GLOBAL_CACHE.lock().unwrap();  // Sync lock in async
    if let Some(data) = cache.get(url) {
        return Ok(data.clone());
    }
    drop(cache);

    let data = reqwest::get(url).await?.bytes().await?.to_vec();

    let mut cache = GLOBAL_CACHE.lock().unwrap();
    cache.insert(url.to_string(), data.clone());
    Ok(data)
}
// Skills needed: rust-async-design (sync lock across await)
// Maybe: rust-error-handling (error propagation)

// SCENARIO 2: Complex generic API with many type parameters
fn process<T, U, V, E>(
    input: T,
    config: U,
    handler: V,
) -> Result<Output, E>
where
    T: Into<String> + Clone + Debug,
    U: AsRef<str> + PartialEq + Eq,
    V: Fn(String) -> Result<Output, E> + Send + Sync,
    E: std::error::Error,
{
    // ...
}
// Skills needed: rust-type-system (over-constrained generics)
// Maybe: rust-design-review (should this be simpler?)

// SCENARIO 3: Struct with too many responsibilities
struct UserManager {
    db: Database,
    cache: Cache,
    auth: AuthService,
    logger: Logger,
    email: EmailService,
    metrics: Metrics,
    // + 10 more fields
}

impl UserManager {
    fn handle_request(&self) { }
    fn process_email(&self) { }
    fn check_cache(&self) { }
    fn sync_db(&self) { }
    // + 40 more methods
}
// Skills needed: rust-architectural-composition-critique (god object)

// SCENARIO 4: Custom error type without proper context
fn load_config(path: &str) -> Result<Config, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("Error: {}", e))?;

    serde_json::from_str(&contents)
        .map_err(|e| format!("Parse error: {}", e))
}
// Skills needed: rust-error-handling (context loss, generic String type)

// SCENARIO 5: Unsafe code without proper documentation
unsafe fn process_pointer(ptr: *const u8) -> u8 {
    *ptr
}
// Skills needed: rust-systems-review (unsafe preconditions, documentation)

// SCENARIO 6: Struct with unnecessary lifetime parameters
struct Document<'a, 'b> {
    title: &'a str,
    content: &'b str,
}

impl<'a, 'b> Document<'a, 'b> {
    fn get_title(&self) -> &'a str {
        self.title
    }
}
// Skills needed: rust-borrowing-complexity (unnecessary lifetimes)

// SCENARIO 7: Trait explosion
trait GetData { fn get(&self) -> Data; }
trait SetData { fn set(&mut self, data: Data); }
trait ValidateData { fn validate(&self, data: &Data) -> bool; }
trait ProcessData { fn process(&self, data: Data) -> Result<Data>; }
trait SerializeData { fn serialize(&self, data: &Data) -> String; }
// Skills needed: rust-architectural-composition-critique (trait explosion)
// Maybe: rust-type-system (design issue)

// SCENARIO 8: Self-referential struct attempt
struct Node {
    value: i32,
    next: Option<Box<Node>>,
}

impl Node {
    fn new(value: i32) -> Self {
        // This works but let's say they try: next: Option<&Node>
        // That would be self-referential and fail
    }
}
// Skills needed: rust-borrowing-complexity (self-reference patterns)

// SCENARIO 9: Code with multiple issues mixed
async fn api_handler(db: Arc<Mutex<Database>>) -> Result<Response, String> {
    // Async with sync lock
    let data = db.lock().unwrap().query().await?;

    // Error context loss
    let processed = process_data(data)
        .map_err(|_| "Error".to_string())?;

    // Generic error type
    Ok(Response { data: processed })
}
// Skills needed:
// 1. rust-async-design (sync lock across await)
// 2. rust-error-handling (String error, context loss)
// 3. rust-systems-review (if there's unsafe elsewhere)

// SCENARIO 10: Over-complex design before implementation
// (From design document, not actual code)
// Skills needed: rust-design-review (pre-implementation validation)

// SCENARIO 11: Code with tight coupling
struct ApiHandler {
    logger: ConsoleLogger,        // Concrete type
    db: PostgresDatabase,         // Concrete type
    auth: LdapAuthService,        // Concrete type
}

impl ApiHandler {
    fn new() -> Self {
        Self {
            logger: ConsoleLogger::new(),
            db: PostgresDatabase::connect("localhost").unwrap(),
            auth: LdapAuthService::new("ldap.example.com").unwrap(),
        }
    }
}
// Skills needed: rust-architectural-composition-critique (tight coupling)
// Maybe: rust-design-review (should have abstracted)

// SCENARIO 12: Missing trait implementations
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn as_tuple(&self) -> (i32, i32) {
        (self.x, self.y)
    }
}

// Could implement AsRef<(i32, i32)>, but doesn't
// Skills needed: rust-trait-detection (missing trait)

// SCENARIO 13: Over-layered architecture
fn handle_request(req: Request) -> Response {
    let parsed = Parser::parse(req);
    let validated = Validator::validate(parsed);
    let authorized = Authorizer::authorize(validated);
    let processed = Processor::process(authorized);
    let formatted = Formatter::format(processed);
    let serialized = Serializer::serialize(formatted);
    Response(serialized)
}
// Skills needed: rust-architectural-composition-critique (over-layering)

// SCENARIO 14: Iterator trait not implemented
struct Items {
    inner: Vec<i32>,
}

impl Items {
    fn into_iter(self) -> std::vec::IntoIter<i32> {
        self.inner.into_iter()
    }
}

// IntoIterator trait not implemented, so can't use in for loop
// Skills needed: rust-advanced-trait-detection (IntoIterator)

// SCENARIO 15: Lifetime issue from borrowing complexity
fn extract<'a, 'b>(items: &'a [&'b str], index: usize) -> &'b str {
    // Could be: fn extract(items: &[&str], index: usize) -> &str
    items[index]
}
// Skills needed: rust-borrowing-complexity (unnecessary explicit lifetimes)

// SCENARIO 16: Mix of type system and composition issues
struct Handler<T, U, V, E>
where
    T: Service,
    U: Repository,
    V: Logger,
    E: std::error::Error,
{
    service: T,
    repo: U,
    logger: V,
    error_type: std::marker::PhantomData<E>,
}

impl<T, U, V, E> Handler<T, U, V, E>
where
    T: Service,
    U: Repository,
    V: Logger,
    E: std::error::Error,
{
    fn handle(&self) -> Result<Response, E> {
        // Implementation
    }
}
// Skills needed:
// 1. rust-type-system (over-genericized, PhantomData unusual)
// 2. rust-architectural-composition-critique (dependency injection pattern)
// 3. Maybe: rust-borrowing-complexity (if lifetime issues exist)
