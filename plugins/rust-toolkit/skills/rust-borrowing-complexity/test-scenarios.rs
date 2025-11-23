// Test scenarios for rust-borrowing-complexity skill
// Complex lifetime patterns, self-referential structs, and borrow patterns

// SCENARIO 1: Unnecessary lifetime parameters
struct Container<'a, T> {
    data: T,
    name: &'a str,
}

impl<'a, T> Container<'a, T> {
    fn get_data(&self) -> &T {
        &self.data  // Doesn't need 'a, T is sufficient
    }
}

// SCENARIO 2: Multiple unrelated lifetimes
fn process<'a, 'b, 'c>(x: &'a str, y: &'b str) -> &'c str {
    // ❌ Three lifetimes, but what's 'c? How does it relate to 'a, 'b?
    x
}

// SCENARIO 3: Self-referential struct (impossible without unsafe/Pin)
struct Node {
    value: i32,
    next: Option<&'static mut Node>,  // ❌ Can't make a linked list this way
}

// SCENARIO 4: Lifetime in struct that could be owned
struct Document<'a> {
    content: &'a str,  // ❌ Why not own the String?
}

// SCENARIO 5: Complex borrow checker issue - multiple references
fn complex_borrow() {
    let mut data = vec![1, 2, 3];
    let r1 = &data[0];
    let r2 = &mut data;  // ❌ Can't have mutable borrow while r1 exists
}

// SCENARIO 6: Lifetime too restrictive
fn extract_first<'a>(items: &'a [&'a str]) -> &'a str {
    // ❌ Inner lifetime too tied to outer
    items[0]
}

// Better:
fn extract_first_better<'a>(items: &[&'a str]) -> &'a str {
    items[0]
}

// SCENARIO 7: Conflicting lifetime constraints
fn merge<'a>(x: &'a str, y: &str) -> &'a str {
    // ❌ Tries to return x with 'a, but what if y is shorter?
    if x.len() > y.len() { x } else { y }
}

// SCENARIO 8: Elided lifetime confusion
fn process_string(s: &str) -> &str {
    // ✅ Actually: fn process_string<'a>(s: &'a str) -> &'a str
    s
}

// SCENARIO 9: Reference to mutable reference
fn modify<'a, 'b>(r: &'a mut &'b mut str) -> &'a mut &'b mut str {
    // ❌ Overly complex - do we need both lifetimes?
    r
}

// SCENARIO 10: Borrow from collection, pattern leads to complexity
fn get_or_default<'a>(map: &'a HashMap<String, String>, key: &str) -> &'a str {
    // ❌ Can't return borrowed ref from map, have to own it
    match map.get(key) {
        Some(v) => v,  // lifetime issue
        None => "default",
    }
}

// SCENARIO 11: Good - clear single lifetime
struct TextBuffer<'a> {
    content: &'a str,
}

impl<'a> TextBuffer<'a> {
    fn process(&self) -> usize {
        self.content.len()
    }
}

// SCENARIO 12: Good - owned data, no lifetime complexity
struct OwnedDocument {
    content: String,
}

impl OwnedDocument {
    fn get_content(&self) -> &str {
        &self.content
    }
}

// SCENARIO 13: Good - lifetime only where needed
fn process_slice<'a>(items: &'a [String]) -> &'a str {
    &items[0]
}

// SCENARIO 14: Lifetime issue with closures
fn apply_callback<'a, F>(data: &'a str, callback: F) -> String
where
    F: Fn(&'a str) -> String,
{
    callback(data)
}

// SCENARIO 15: Self-referential with Pin (correct approach)
use std::pin::Pin;

struct SelfReferential {
    name: String,
    reference: Option<*const String>,  // Raw pointer to self.name
}

impl SelfReferential {
    fn new(name: String) -> Self {
        let mut s = SelfReferential {
            name,
            reference: None,
        };
        s.reference = Some(&s.name as *const _);
        s
    }
}

// SCENARIO 16: Unnecessary lifetime in where clause
fn process<T: std::fmt::Display + 'static>(t: T) {
    // ❌ 'static isn't needed for Display
    println!("{}", t);
}

// SCENARIO 17: Complex generic + lifetime combo
struct Complex<'a, T, U, V>
where
    T: std::fmt::Debug + 'a,
    U: std::fmt::Display,
    V: std::fmt::Write + 'a,
{
    data: &'a T,
    output: U,
    writer: V,
}

// ❌ Is this complexity justified?

// SCENARIO 18: Lifetime elision at work
fn first_word(s: &str) -> &str {
    // ✅ Elided lifetimes make this simple
    s.split(' ').next().unwrap_or(s)
}

// Explicit version would be:
fn first_word_explicit<'a>(s: &'a str) -> &'a str {
    s.split(' ').next().unwrap_or(s)
}

// SCENARIO 19: Stringly-typed to avoid lifetime complexity
fn format_report(name: String, age: i32) -> String {
    // ❌ Takes owned String to avoid lifetime params
    format!("{} is {}", name, age)
}

// Better:
fn format_report_better(name: &str, age: i32) -> String {
    format!("{} is {}", name, age)
}

// SCENARIO 20: Good - borrows with clear relationships
struct Ctx<'a> {
    data: &'a [u8],
}

fn process_ctx<'a>(ctx: &'a Ctx<'a>) -> &'a [u8] {
    ctx.data
}
