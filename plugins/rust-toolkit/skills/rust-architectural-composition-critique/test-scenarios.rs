// Test scenarios for rust-architectural-composition-critique skill
// Struct composition, layering, trait usage, and architectural patterns

// SCENARIO 1: God struct - everything in one struct
struct Application {
    database: Database,
    cache: Cache,
    auth: AuthService,
    logger: Logger,
    config: Config,
    metrics: Metrics,
    queue: MessageQueue,
    email_service: EmailService,
    api_handlers: Vec<Box<dyn ApiHandler>>,
    // ❌ 10+ responsibilities, impossible to test, reuse, or reason about
}

// SCENARIO 2: Over-layered architecture
struct Request;
struct RequestValidator;
struct RequestProcessor;
struct RequestTransformer;
struct RequestExecutor;
struct RequestFinalizer;

// ❌ Six layers for one operation, each passing data to next
// Tight coupling, hard to test, unclear responsibility

// SCENARIO 3: Trait per method - explosion of traits
trait GetData {
    fn get_data(&self) -> Data;
}

trait SetData {
    fn set_data(&self, data: Data);
}

trait ProcessData {
    fn process(&self, data: Data) -> Result<Data>;
}

trait ValidateData {
    fn validate(&self, data: &Data) -> bool;
}

// ❌ Four traits for one concept (data management)
// Callers must implement all traits, imports become messy

// SCENARIO 4: Composition with unnecessary indirection
struct User {
    profile: UserProfile,
}

struct UserProfile {
    details: UserDetails,
}

struct UserDetails {
    name: String,
}

// To access name: user.profile.details.name
// ❌ Unnecessary nesting adds complexity without benefit

// SCENARIO 5: Generic over everything (over-engineered)
struct Container<T, U, V, W, X>
where
    T: Trait1,
    U: Trait2,
    V: Trait3,
    W: Trait4,
    X: Trait5,
{
    // ...
}

// ❌ Five type parameters for something that could have 1-2

// SCENARIO 6: Trait with too many methods - fat interface
trait Repository: Clone + Send + Sync {
    fn find_by_id(&self, id: u32) -> Result<Entity>;
    fn find_all(&self) -> Result<Vec<Entity>>;
    fn find_by_name(&self, name: &str) -> Result<Vec<Entity>>;
    fn find_by_date_range(&self, from: Date, to: Date) -> Result<Vec<Entity>>;
    fn insert(&mut self, entity: Entity) -> Result<u32>;
    fn update(&mut self, id: u32, entity: Entity) -> Result<()>;
    fn delete(&mut self, id: u32) -> Result<()>;
    fn count(&self) -> Result<u64>;
    fn exists(&self, id: u32) -> Result<bool>;
    // ❌ 9+ methods, should be split or composed
}

// SCENARIO 7: Struct composition hiding abstraction
struct Handler {
    database: Box<dyn Repository>,
    cache: Box<dyn CacheService>,
    auth: Box<dyn AuthService>,
}

impl Handler {
    fn process(&self, req: Request) -> Response {
        // How is database used? Cache? Auth?
        // Tight coupling to implementation details
        // Hard to test with different implementations
        // ❌ Unclear boundaries and dependencies
    }
}

// SCENARIO 8: Tight coupling between components
struct Service {
    logger: ConcreteLogger,  // ❌ Concrete type, not trait
    db: DatabaseConnection,  // ❌ Concrete, can't mock
}

// Impossible to test Service without real database

// SCENARIO 9: Deep inheritance-like trait stacking
trait Base {
    fn base_op(&self);
}

trait Derived: Base {
    fn derived_op(&self);
}

trait MoreDerived: Derived {
    fn more_op(&self);
}

// ❌ Three-level trait hierarchy, single inheritance problem

// SCENARIO 10: Good composition - clear separation
struct App {
    handlers: ApiHandlers,
    services: AppServices,
    persistence: Persistence,
}

struct ApiHandlers {
    users: UsersHandler,
    products: ProductsHandler,
}

struct AppServices {
    auth: AuthService,
    validation: ValidationService,
}

struct Persistence {
    users: UsersRepository,
    products: ProductsRepository,
}

// ✅ Clear structure, can test each piece

// SCENARIO 11: Over-abstraction - trait for single implementor
trait LoggerBackend {
    fn write(&self, msg: &str);
}

struct ConsoleLoggerBackend;
impl LoggerBackend for ConsoleLoggerBackend {
    fn write(&self, msg: &str) {
        println!("{}", msg);
    }
}

// ❌ One trait for one impl - no benefit

// SCENARIO 12: Composition with cyclic dependency risk
struct UserService {
    auth: Arc<AuthService>,
}

struct AuthService {
    user_service: Arc<UserService>,  // ❌ Cycle risk
}

// SCENARIO 13: Good separation - clear boundaries
struct UserRepository {
    db: Arc<Database>,
}

struct AuthService {
    users: Arc<UserRepository>,
}

// ✅ Clear one-way dependency

// SCENARIO 14: Composition pattern - wrapper vs original
struct Json(serde_json::Value);  // ❌ Newtype adds no value

struct Config {
    json: serde_json::Value,  // Better: just use Value directly
}

// SCENARIO 15: Good newtype - adds semantic meaning
struct UserId(u32);  // ✅ Prevents mixing with other u32s
struct Timestamp(u64);  // ✅ Clear intent

// SCENARIO 16: Enum vs trait confusion
trait Animal {
    fn speak(&self) -> String;
}

struct Dog;
impl Animal for Dog {
    fn speak(&self) -> String { "woof".into() }
}

// ❌ Could just be an enum if not many implementations:
enum AnimalEnum {
    Dog,
    Cat,
}

// SCENARIO 17: Good enum when variants are fixed
#[derive(Debug)]
enum Result<T, E> {
    Ok(T),
    Err(E),
}

// ✅ Fixed set of variants, best as enum

// SCENARIO 18: Builder pattern vs simple struct
struct ComplexConfig;

impl ComplexConfig {
    fn new() -> ConfigBuilder { ConfigBuilder::default() }
}

struct ConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    timeout: Option<Duration>,
}

// ❌ Builder overkill if all fields are just optional

// SCENARIO 19: Good builder - complex validation
struct DatabaseConfig;

impl DatabaseConfig {
    fn new() -> ConfigBuilder { ConfigBuilder::default() }
}

struct ConfigBuilder {
    host: String,
    port: u16,
    username: String,
    password: String,
}

impl ConfigBuilder {
    fn validate(self) -> Result<DatabaseConfig, ConfigError> {
        // Validate host is not localhost in production
        // Validate password meets security requirements
        // etc
        todo!()
    }
}

// ✅ Builder with validation

// SCENARIO 20: Over-composition with too much delegation
struct RequestHandler;

impl RequestHandler {
    fn handle(&self, req: Request) -> Response {
        let parsed = RequestParser.parse(req);
        let validated = RequestValidator.validate(parsed);
        let authorized = AuthorizationChecker.check(validated);
        let processed = RequestProcessor.process(authorized);
        let serialized = ResponseSerializer.serialize(processed);
        serialized
    }
}

// ❌ Chain of single-purpose objects adds complexity
// Better: RequestHandler contains all logic or owns smaller pieces
