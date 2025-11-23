# rust-design-review Test Scenarios

These represent real design documents that should trigger the skill's review process.

## SCENARIO 1: Over-engineered async architecture

```
# Message Queue System Design

## Architecture
- AsyncIO runtime handling all I/O
- Message queue with custom async executor
- Trait objects for message handlers
- Multiple trait bounds for flexibility
- Pub/sub system with dynamic dispatch

## Components
- `MessageBroker<T: Handler + Send + Sync + Debug>`
- `AsyncDispatcher` for runtime handling
- `HandlerRegistry` storing `Vec<Box<dyn Handler>>`
```

**Should trigger questions:**
- Why custom async executor vs tokio/async-std?
- Are multiple handler implementors needed or is trait over-engineered?
- Does the Handler trait have stable interface?
- Could this be simpler with concrete types?

## SCENARIO 2: Missing error strategy

```
# User Authentication Module

## Components
- `LoginService` for credential validation
- `TokenGenerator` for JWT creation
- `UserRepository` for database access

## Error Handling
- Uses `Result<T, E>` throughout
- Error types not specified
- No error context or recovery strategy documented
```

**Should trigger questions:**
- What errors can occur at each layer?
- How should errors be propagated vs handled?
- Should errors provide context?
- Can callers recover from specific errors?

## SCENARIO 3: Unclear data flow

```
# Real-time Analytics Pipeline

## Design
- Data ingestion from multiple sources
- Processing through transformation chain
- Aggregation for reporting

## Unspecified:
- How do sources deliver data? (push/pull)
- How are transformations composed?
- What happens on transformation failure?
- How is state managed?
```

**Should trigger questions:**
- What's the actual data flow? Draw it.
- Which components are sync vs async?
- Where can failures occur?
- Is state mutable and how is it protected?

## SCENARIO 4: Unclear boundaries and dependencies

```
# Service Configuration System

## Components
- ConfigLoader
- ConfigValidator
- ConfigCache
- ConfigWatcher

## Unclear:
- What owns what?
- How do components communicate?
- Initialization order?
- Can a ConfigValidator be used standalone?
```

**Should trigger questions:**
- Draw a dependency diagram
- What's the ownership model?
- Which components are optional?
- Can components be composed differently?

## SCENARIO 5: Performance assumptions not validated

```
# High-throughput Data Processing

## Goals
- Process millions of events per second
- Sub-millisecond latency
- 99.9% availability

## Design Details
- Uses shared state with Arc<Mutex<T>>
- Background thread polls for updates
- No mention of allocation strategy
- No benchmarking plan documented
```

**Should trigger questions:**
- Are Arc<Mutex<T>> appropriate for this scale?
- What's the actual latency target vs assumed?
- How will you measure throughput?
- Should we use lock-free structures?

## SCENARIO 6: Good design (no major issues)

```
# Logging Facade Design

## Purpose
Provide unified logging interface over multiple backends

## Architecture
- Single `Logger` trait with stable interface
- Concrete implementations: Console, File, Syslog
- No dynamic dispatch in hot paths
- Initialization happens once at startup

## Error Strategy
- Logger never fails (writes to stderr as fallback)
- No error propagation needed

## Data Flow
- Application calls `log(message)`
- Facade routes to configured backend
- Each backend writes synchronously or enqueues

## Testing
- Mock implementation for unit tests
- Integration tests with real backends
```

**Should NOT trigger warnings:**
- Design is straightforward
- Error handling is explicit
- Trait usage justified
- No unnecessary complexity
