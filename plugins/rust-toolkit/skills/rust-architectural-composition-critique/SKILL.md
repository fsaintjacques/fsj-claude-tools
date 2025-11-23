---
name: rust-architectural-composition-critique
description: Review Rust struct composition, trait design, and architectural choices - identifies god objects, over-layering, trait explosion, unnecessary abstraction, and composition problems through structural analysis
---

# Rust Architectural Composition Critique

## Overview

Review Rust code's architectural structure: how structs compose, how traits are used, how components relate. Poor composition creates complexity, testing nightmares, and tight coupling.

**Core principle:** Composition should clarify structure, not hide it.

**Use when:** Reviewing struct design, trait usage, component boundaries, or architectural choices.

**Do NOT use this skill for:**
- Type system design alone (use `rust-type-system`)
- Pre-implementation design (use `rust-design-review`)
- Borrow complexity (use `rust-borrowing-complexity`)

## The Composition Analysis Process

### Category 1: God Objects - Too Many Responsibilities

**The Problem:**
A single struct has too many fields, too many methods, manages too many concerns. Impossible to test, understand, or reuse.

**Pattern: God object**
```rust
// ❌ One struct doing everything
struct Application {
    database: Database,
    cache: Cache,
    auth: AuthService,
    logger: Logger,
    config: Config,
    metrics: Metrics,
    queue: MessageQueue,
    email: EmailService,
    api_handlers: Vec<Handler>,
    // + 20 more fields
}

impl Application {
    fn handle_request(&self, req: Request) -> Response { }
    fn process_queue(&self) { }
    fn sync_cache(&self) { }
    fn send_email(&self) { }
    fn log_metrics(&self) { }
    // + 30 more methods
}
```

**Questions to ask:**
- How many distinct responsibilities does this struct have?
- Could you test just one responsibility in isolation?
- Can callers use just one part of this struct?
- How many methods does it have?
- How many dependencies does it own?

**Red flags:**
- 10+ fields that are unrelated
- 20+ methods
- Constructor takes many parameters
- Methods operate on disjoint subsets of fields
- Hard to describe what the struct does in one sentence
- Testing requires initializing all dependencies

**How to fix: Decompose into smaller pieces**
```rust
// ✅ Separated concerns
struct App {
    api: ApiServer,
    persistence: PersistenceLayer,
    messaging: MessagingService,
}

struct ApiServer {
    handlers: Vec<Box<dyn Handler>>,
    auth: Arc<AuthService>,
}

struct PersistenceLayer {
    database: Arc<Database>,
    cache: Arc<Cache>,
}

struct MessagingService {
    queue: Arc<MessageQueue>,
    email: Arc<EmailService>,
}

// Each struct has clear responsibility
// Test each independently
// Compose only what's needed
```

### Category 2: Over-Layering - Too Many Abstraction Levels

**The Problem:**
Data passes through 4, 5, 6 layers before being processed. Each layer adds minimal value but maximum complexity.

**Pattern: Over-layered architecture**
```rust
// ❌ Six layers for one operation
fn handle_request(req: Request) -> Response {
    let parsed = Parser::parse(req);           // Layer 1
    let validated = Validator::validate(parsed);  // Layer 2
    let authorized = Auth::check(validated);     // Layer 3
    let processed = Processor::process(authorized); // Layer 4
    let formatted = Formatter::format(processed);   // Layer 5
    let serialized = Serializer::serialize(formatted); // Layer 6
    Response(serialized)
}

// Each layer is a struct, each operation delegates to next
// Hard to trace data flow
// Hard to understand purpose of each layer
```

**Questions to ask:**
- How many layers does data pass through?
- What value does each layer add?
- Could multiple layers be combined?
- Is each layer meaningful or just ceremony?
- Can you test layer N without layers N-1 through N-5?

**Red flags:**
- 5+ layers in processing pipeline
- Each layer has one method that delegates
- Data shape changes at each layer
- Can't modify layer without updating all others
- Comments explaining the layering (usually a sign it's unclear)

**How to fix: Reduce layering**
```rust
// ✅ Direct, clear path
fn handle_request(req: Request) -> Response {
    let req = Parser::parse(req)?;
    Validator::validate(&req)?;
    Auth::check(&req)?;

    let processed = Processor::process(req)?;
    Response::from(processed)
}

// Or consolidate related operations
impl RequestHandler {
    fn handle(&self, req: Request) -> Result<Response> {
        self.validate(&req)?;
        self.authorize(&req)?;
        let result = self.process(req)?;
        Ok(Response::from(result))
    }
}
```

### Category 3: Trait Explosion - Too Many Traits

**The Problem:**
One concept split across 5, 6, 7 traits. Callers must implement all traits. Imports become a mess.

**Pattern: Trait explosion**
```rust
// ❌ One concept, many traits
trait GetData {
    fn get_data(&self) -> Data;
}

trait SetData {
    fn set_data(&mut self, data: Data);
}

trait ValidateData {
    fn validate(&self, data: &Data) -> bool;
}

trait ProcessData {
    fn process(&self, data: Data) -> Result<Data>;
}

trait SerializeData {
    fn serialize(&self, data: &Data) -> String;
}

// To implement: must implement all 5 traits
// Callers must know about all 5 traits
// Conceptually related but scattered
```

**Questions to ask:**
- Do these traits represent one concept or many?
- Would a single trait with multiple methods be simpler?
- Do all implementors implement all traits?
- Are callers confused by which trait to use?
- Could methods be grouped meaningfully?

**Red flags:**
- Trait with one method
- Multiple single-method traits for related operations
- Caller code with 5+ trait bounds
- Traits that are always implemented together
- Import statements cluttered with trait names

**How to fix: Consolidate traits**
```rust
// ✅ One coherent trait
trait DataStore {
    fn get(&self) -> Result<Data>;
    fn set(&mut self, data: Data) -> Result<()>;
    fn validate(&self, data: &Data) -> Result<()>;
    fn process(&self, data: Data) -> Result<Data>;
    fn serialize(&self, data: &Data) -> Result<String>;
}

// Or split by client need
trait DataReader {
    fn get(&self) -> Result<Data>;
    fn serialize(&self, data: &Data) -> Result<String>;
}

trait DataWriter {
    fn set(&mut self, data: Data) -> Result<()>;
    fn validate(&self, data: &Data) -> Result<()>;
}
```

### Category 4: Unnecessary Indirection - Extra Levels Without Benefit

**The Problem:**
Data wrapped in extra structs that add no semantic value. Accessing fields requires 5 levels of nesting.

**Pattern: Unnecessary nesting**
```rust
// ❌ Excessive nesting adds no value
struct User {
    profile: UserProfile,
}

struct UserProfile {
    details: UserDetails,
}

struct UserDetails {
    personal: PersonalInfo,
}

struct PersonalInfo {
    name: String,
}

// To access: user.profile.details.personal.name
// What does each layer add conceptually?
```

**Questions to ask:**
- Does each wrapper layer add semantic meaning?
- Would the code be clearer with fewer levels?
- Are there operations specific to each level?
- Is this layering hiding important concerns or just ceremony?

**Red flags:**
- Wrapper structs with single field
- 4+ levels deep just to reach actual data
- Each level's methods just delegate to next
- No validation or behavior at intermediate levels
- Comments saying "this is just a wrapper"

**How to fix: Flatten unnecessary levels**
```rust
// ✅ Direct structure with meaning
struct User {
    name: String,
    email: String,
    bio: String,
}

// Or if some grouping makes sense:
struct User {
    contact: Contact,  // Meaningful grouping
    profile: Profile,  // Meaningful grouping
}

struct Contact {
    email: String,
    phone: String,
}

struct Profile {
    bio: String,
    avatar_url: String,
}
```

### Category 5: Over-Abstraction - Too Generic for the Use Case

**The Problem:**
Designed as maximally generic when concrete types would be simpler. Many type parameters, traits, and "flexibility" that isn't used.

**Pattern: Over-engineered generics**
```rust
// ❌ Five type parameters for something that needs 1
struct Container<T, E, F, G, H>
where
    T: Trait1,
    E: Trait2,
    F: Trait3,
    G: Trait4,
    H: Trait5,
{
    // ...
}

// Callers must understand all five generics
// Only one combination is ever used in practice
```

**Questions to ask:**
- How many type parameters does this have?
- How many concrete uses of this struct exist?
- If only one, why is it generic?
- What flexibility is gained vs. complexity added?
- Could a trait object replace the generics?

**Red flags:**
- 4+ type parameters without documented reason
- Only one concrete instantiation in the codebase
- Comments saying "might need to support X in the future"
- Generic over things that never change
- Complexity justified by "flexibility"

**How to fix: Use concrete types unless truly needed**
```rust
// ✅ Only generic where actually used
struct Logger<W: Write> {
    writer: W,
}

// Concrete, simpler version
struct FileLogger {
    file: File,
}

// Or use trait objects if multiple implementors
struct Logger {
    writer: Box<dyn Write>,
}
```

### Category 6: Fat Trait Interfaces - Too Many Methods

**The Problem:**
Trait with 9+ methods. Implementors must implement everything even if they only use 1-2. Callers confused which methods to use.

**Pattern: Fat interface**
```rust
// ❌ One trait with too many methods
trait Repository {
    fn find_by_id(&self, id: u32) -> Result<Entity>;
    fn find_all(&self) -> Result<Vec<Entity>>;
    fn find_by_name(&self, name: &str) -> Result<Vec<Entity>>;
    fn find_by_date(&self, date: Date) -> Result<Vec<Entity>>;
    fn insert(&mut self, entity: Entity) -> Result<u32>;
    fn update(&mut self, id: u32, entity: Entity) -> Result<()>;
    fn delete(&mut self, id: u32) -> Result<()>;
    fn count(&self) -> Result<u64>;
    fn exists(&self, id: u32) -> Result<bool>;
    fn bulk_insert(&mut self, entities: Vec<Entity>) -> Result<Vec<u32>>;
    // + more...
}

// Implementors must implement all even if they only need 3
// Callers must understand all methods
```

**Questions to ask:**
- How many methods does this trait have?
- Do all implementors need all methods?
- Could this trait be split into reader/writer?
- Could this be composed from smaller traits?
- Are there methods that only some implementors need?

**Red flags:**
- Trait with 8+ methods
- Methods that operate on disjoint sets of state
- Some methods marked "not implemented" or panicking
- Implementors providing dummy implementations
- Comments listing which methods apply when

**How to fix: Split or compose traits**
```rust
// ✅ Split by concern
trait RepositoryRead {
    fn find_by_id(&self, id: u32) -> Result<Entity>;
    fn find_all(&self) -> Result<Vec<Entity>>;
    fn exists(&self, id: u32) -> Result<bool>;
}

trait RepositoryWrite {
    fn insert(&mut self, entity: Entity) -> Result<u32>;
    fn update(&mut self, id: u32, entity: Entity) -> Result<()>;
    fn delete(&mut self, id: u32) -> Result<()>;
}

trait Repository: RepositoryRead + RepositoryWrite {}

// Or use focused single traits
trait Queryable {
    fn find_by_id(&self, id: u32) -> Result<Entity>;
}

trait Writable {
    fn insert(&mut self, entity: Entity) -> Result<u32>;
}
```

### Category 7: Tight Coupling - Hard to Test or Reuse

**The Problem:**
Components hardwired to concrete types. Can't test without real dependencies. Can't reuse in different context.

**Pattern: Tight coupling**
```rust
// ❌ Hardwired to concrete types
struct Handler {
    logger: ConsoleLogger,        // Concrete type
    db: PostgresConnection,       // Concrete type
    auth: LdapAuthService,        // Concrete type
}

impl Handler {
    fn process(&self, req: Request) -> Response {
        self.logger.log(&req);
        let user = self.auth.authenticate(req.token);
        let data = self.db.query(...);
        // ...
    }
}

// Can't test Handler without real database, real LDAP, real logging
// Can't use Handler with different database
// Impossible to mock dependencies
```

**Questions to ask:**
- Are dependencies concrete types or traits?
- Can you test this without real dependencies?
- Could you swap implementations?
- Is there trait abstraction or hardwired concrete?
- Are dependencies owned or injected?

**Red flags:**
- Concrete types as fields (not Arc<T>, not dyn Trait)
- New/constructor creates dependencies internally
- Can't construct with test doubles
- Dependencies are complex (Database, NetworkClient)
- Tests require setup of actual external services

**How to fix: Abstract dependencies with traits**
```rust
// ✅ Traits instead of concrete types
trait Logger: Send + Sync {
    fn log(&self, msg: &str);
}

trait AuthService: Send + Sync {
    fn authenticate(&self, token: &str) -> Result<User>;
}

trait Database: Send + Sync {
    fn query(&self, sql: &str) -> Result<Vec<Row>>;
}

struct Handler<L, A, D>
where
    L: Logger,
    A: AuthService,
    D: Database,
{
    logger: Arc<L>,
    auth: Arc<A>,
    db: Arc<D>,
}

// Or use trait objects
struct Handler {
    logger: Arc<dyn Logger>,
    auth: Arc<dyn AuthService>,
    db: Arc<dyn Database>,
}

// Now you can test with mocks/stubs
```

### Category 8: Trait Hierarchy - Single Inheritance Problems

**The Problem:**
Three-level trait hierarchy where one depends on another. Violates composition principle. Reminds of class inheritance.

**Pattern: Trait hierarchy**
```rust
// ❌ Three-level trait hierarchy
trait Base {
    fn base_op(&self);
}

trait Derived: Base {
    fn derived_op(&self);
}

trait MoreDerived: Derived {
    fn more_op(&self);
}

// Implementor must satisfy entire chain
// Difficult to implement just Derived without MoreDerived
// Looks like inheritance (bad in Rust)
```

**Questions to ask:**
- Is this trait hierarchy justified?
- Is it inheritance-like (bad) or composition-like (good)?
- Could these be separate traits composed by callers?
- Would a single trait with conditional methods be clearer?
- Are all levels always needed together?

**Red flags:**
- More than 2 levels of trait dependency
- Implementing lower trait requires implementing all upper
- Hierarchy not justified in comments
- Clients confused which trait to use
- Can't use Base without also implementing Derived

**How to fix: Flatten or compose**
```rust
// ✅ Separate traits, composed by caller
trait Operation {
    fn op(&self);
}

trait Validation {
    fn validate(&self) -> bool;
}

trait Processing {
    fn process(&self) -> Result<Data>;
}

// Implementor provides what's needed
struct Service;
impl Operation for Service { }
impl Validation for Service { }
impl Processing for Service { }

// Callers use what they need
fn handle<T: Operation + Processing>(t: &T) { }
```

## The Composition Checklist

When reviewing architectural composition:

### Structure and Boundaries
- [ ] Struct has clear, single responsibility
- [ ] Field count is reasonable (5-10 usually, rarely 15+)
- [ ] Method count is reasonable (5-20 usually)
- [ ] Can describe the struct's purpose in one sentence
- [ ] Components can be understood independently

### Layering
- [ ] Processing pipeline has 3-4 layers max
- [ ] Each layer adds clear value
- [ ] Data flow is traceable
- [ ] Layers aren't just passthrough
- [ ] Testing doesn't require all layers

### Trait Design
- [ ] Traits have 1-8 methods (rarely more)
- [ ] Methods in trait are related
- [ ] Not trait-per-method
- [ ] Trait can be implemented by multiple types
- [ ] Implementors use all (or most) methods

### Abstraction Level
- [ ] Generics used where actually needed
- [ ] Not overengineered for theoretical future
- [ ] Type parameters justified
- [ ] Concrete types used when appropriate
- [ ] Complexity serves a purpose

### Dependencies
- [ ] Dependencies are injected (not created internally)
- [ ] Can swap implementations (testable)
- [ ] Uses traits not concrete types
- [ ] Dependency graph is acyclic
- [ ] Can test with mocks/stubs

### Nesting
- [ ] Nesting depth is 2-3 levels usually
- [ ] Each level adds semantic meaning
- [ ] Not just wrapper structs
- [ ] Flattening would lose information
- [ ] Accessor methods provided where needed

## Red Flags Requiring Immediate Attention

- [ ] Struct with 15+ fields
- [ ] Struct with 30+ methods
- [ ] 5+ layer processing pipeline
- [ ] Single-method traits
- [ ] 10+ method trait
- [ ] Concrete type dependencies (should be traits)
- [ ] Three-level trait hierarchy
- [ ] 5+ type parameters
- [ ] Can't test without real external services
- [ ] Excessive nesting (4+ levels)

## Common Architectural Patterns

### Pattern 1: Layered (but not over-layered)
```rust
// ✅ Reasonable three-layer architecture
struct Api {
    handlers: Vec<Handler>,
}

struct Handler {
    service: Arc<Service>,
}

struct Service {
    repository: Arc<dyn Repository>,
}

// Each layer adds value, testable, clear
```

### Pattern 2: Composed Services
```rust
// ✅ Services composed by larger service
struct App {
    users: UserService,
    auth: AuthService,
    notifications: NotificationService,
}

// Each service handles one domain
// App composes them
// Each testable independently
```

### Pattern 3: Trait-Based Flexibility
```rust
// ✅ Traits where flexibility matters
trait Storage {
    fn get(&self, key: &str) -> Result<Value>;
    fn set(&mut self, key: String, value: Value) -> Result<()>;
}

// Multiple implementors: FileStorage, MemoryStorage, CloudStorage
// Caller chooses implementation
```

### Pattern 4: Simple Composition
```rust
// ✅ Straightforward composition
struct Config {
    database: DatabaseConfig,
    server: ServerConfig,
    logging: LoggingConfig,
}

// Groups related settings
// Each section makes sense independently
// Not over-fragmented
```

## Decision Tree for Reviewing

```
Does the struct have too many responsibilities?

  Yes?
  ├─ Fields: 15+? → Split into smaller structs
  ├─ Methods: 30+? → Move methods to domain-specific types
  └─ Use case: Does it do 3+ unrelated things? → Decompose

Is there over-layering?

  Yes?
  ├─ Layers: 5+? → Consolidate related layers
  ├─ Each layer adds value? → No? → Merge layers
  └─ Can test layer N without layers N-1 through N-5? → No? → Flatten

Does the trait have too many methods?

  Yes?
  ├─ Methods: 8+? → Split into smaller traits
  ├─ Do all methods operate on same state? → No? → Separate concerns
  └─ Do all implementors need all methods? → No? → Split trait

Is composition over-abstracted?

  Yes?
  ├─ Type parameters: 4+? → Use concrete types where possible
  ├─ Only one use case? → Make it concrete
  └─ "Future flexibility" without concrete use? → Remove generics

Are dependencies testable?

  No?
  ├─ Using concrete types? → Abstract with traits
  ├─ Creating dependencies internally? → Inject dependencies
  └─ Can't mock? → Redesign for testability
```

## Example: God Object (Before/After)

**Before: God Object**
```rust
struct Application {
    db: Database,
    cache: Cache,
    auth: AuthService,
    email: EmailService,
    logger: Logger,
    config: Config,
    queue: Queue,
    metrics: Metrics,
    // + 20 more fields
}

// 50+ methods, unclear responsibility
```

**After: Decomposed**
```rust
struct App {
    api: ApiLayer,
    persistence: PersistenceLayer,
    services: ApplicationServices,
}

struct ApiLayer {
    handlers: Vec<Box<dyn Handler>>,
    auth: Arc<AuthService>,
}

struct PersistenceLayer {
    primary: Arc<Database>,
    cache: Arc<Cache>,
}

struct ApplicationServices {
    email: Arc<EmailService>,
    queue: Arc<Queue>,
}

// Clear responsibility, testable, composable
```

## Example: Over-Layering (Before/After)

**Before: Six Layers**
```rust
fn process(req: Request) -> Response {
    let parsed = Parser::parse(req);
    let validated = Validator::validate(parsed);
    let authorized = Auth::check(validated);
    let processed = Processor::process(authorized);
    let formatted = Formatter::format(processed);
    let serialized = Serializer::serialize(formatted);
    Response(serialized)
}
```

**After: Consolidated**
```rust
fn process(req: Request) -> Result<Response> {
    let parsed = Parser::parse(req)?;
    Validator::validate(&parsed)?;
    Auth::check(&parsed)?;
    let processed = Processor::process(parsed)?;
    Ok(Response::from(processed))
}
```
