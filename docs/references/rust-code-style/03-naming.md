# 3. Naming

> Part of the **Rust Code Style**. See [`index.md`](./index.md) for the scope and the meaning of
> **MUST / SHOULD / MAY**.

## 3.1 General conventions

| Item | Convention | Examples |
|---|---|---|
| Crates, modules, functions, methods, macros, local variables, parameters | `snake_case` | `parse_value`, `render_call`, `start_instance`, `ctx`, `workflow_src` |
| Structs, enums, traits, type aliases, enum variants | `PascalCase` | `Value`, `Context`, `Status`, `Instr`, `StepOutcome`, `Spanned` |
| Constants and immutable statics | `UPPER_SNAKE_CASE` | `VALUE_NODES`, `WORKFLOW_FILE`, `CONTEXT_FILE` |
| Lifetimes | short lowercase names | `'a`, `'de`, `'ctx` |
| Generic type parameters | concise `PascalCase` | `T` (e.g. `Spanned<T>`) |
| Boolean values and methods | predicate names | `is_running`, `truthy` |

Treat acronyms as normal words.

```rust
// Incorrect
struct CLI;
struct IRAst;
fn parse_JSON(input: &str) {}

// Correct
struct Cli;
struct IrAst;
fn parse_json(input: &str) {}
```

## 3.2 Domain names

Names MUST describe the domain role, not the current implementation technique.

```rust
// Incorrect
struct StringMap;
struct HashMapThing;
fn do_it(expr: &Expr) -> Value;

// Correct: the name describes the domain role.
struct VariableBindings; // a scope of name -> Value bindings
fn eval(expr: &Expr, vars: &HashMap<String, Value>) -> Result<Value, EvalError>;
```

Use nouns for values and types, and verbs for operations.

```rust
// Correct
struct Context;   // the resumable run state
struct Frame;     // one call-stack entry
struct StepState; // per-step state

fn lower(module: &Module) -> Vec<Instr>;
fn step(ir: &[Instr], ctx: &mut Context) -> StepOutcome;
```

Avoid vague terms unless the surrounding scope makes them precise.

```rust
// Avoid in public APIs.
fn handle(value: Value);
fn process(data: Data);
fn run_thing(item: Item);
```

Use names that reveal the produced kind of result.

```rust
// A predicate answers a boolean question.
impl Context {
    pub fn is_running(&self) -> bool { /* ... */ }
}

impl Value {
    // Produces text for interpolation.
    pub fn render(&self) -> String { /* ... */ }
    // Answers the truthiness used by branching.
    pub fn truthy(&self) -> bool { /* ... */ }
}
```

## 3.3 Getters and boolean methods

Prefer exposing plain state as **public fields** (`ctx.pc`, `ctx.status`, `ctx.vars`) and reserve
methods for computed results. Do not use `get_` for ordinary accessors; when a method is a
computed boolean, name it as a predicate.

```rust
// Incorrect: a `get_` accessor for a derived boolean.
impl Context {
    pub fn get_running(&self) -> bool {
        self.status == Status::Running
    }
}

// Correct: a predicate reads as a yes/no question.
impl Context {
    pub fn is_running(&self) -> bool {
        self.status == Status::Running
    }
}
```

Use `get` for runtime-checked lookup operations.

```rust
// A map lookup that may fail at runtime.
pub fn get_step(&self, pc: u32) -> Option<&StepState> {
    self.steps.get(&pc)
}
```

Boolean methods MUST read as predicates.

```rust
// Incorrect
fn completed(&self) -> bool;
fn failed(&self) -> bool;

// Correct
fn is_complete(&self) -> bool;
fn is_failed(&self) -> bool;
```

## 3.4 Conversion names

Use Rust's standard conversion vocabulary.

| Prefix | Meaning | Examples |
|---|---|---|
| `as_` | cheap borrowed view or representation change | `as_str`, `as_path` |
| `to_` | creates or computes another representation | `Value::render` (to text), `to_string` |
| `into_` | consumes `self` to produce another value | `into_inner`, `into_parts` |
| `from_` | named associated conversion constructor | `from_str` |
| `try_from_` | fallible named conversion constructor | `try_from_env` |

A `parse(&str) -> Result<Self, E>` constructor parses text into a typed value and follows the
`try_from_` convention.

```rust
// Incorrect
fn make_string(&self) -> String;
fn take_parts(self) -> Parts;

// Correct
fn to_string(&self) -> String;
fn into_parts(self) -> Parts;
```

Use `From` or `TryFrom` when the conversion is idiomatic and the source/target types make the
meaning clear. Error enums commonly derive the `From` conversions via thiserror `#[from]`.

## 3.5 Constructors

Use `new()` for the obvious primary constructor.

```rust
// Correct: the obvious, no-argument start state.
impl Context {
    #[must_use]
    pub fn new() -> Self {
        Context {
            pc: 0,
            status: Status::Running,
            vars: HashMap::new(),
            frames: Vec::new(),
            steps: HashMap::new(),
        }
    }
}
```

Use a domain verb when construction has a meaningful action or source.

```rust
// Correct: construction is "parse this text".
let tmpl = Template::parse(source)?;
let module = parse(&src)?;
start_instance(&dir, &workflow_src)?;
```

Use a builder when construction has several optional, independent parameters.

```rust
// Incorrect
let opts = Options::new(timeout, retries, output_dir, dry_run, verbose);

// Correct
let opts = Options::builder()
    .timeout(timeout)
    .retries(retries)
    .output_dir(output_dir)
    .dry_run(dry_run)
    .build()?;
```
