# 4. Types, APIs, and Data Modeling

> Part of the **Rust Code Style**. See [`index.md`](./index.md) for the scope and the meaning of
> **MUST / SHOULD / MAY**.

## 4.1 Prefer domain types over primitive bundles

Use newtypes for values whose meaning must not be confused. Where two raw integers (or strings)
play different domain roles, wrap them so they cannot be passed in the wrong slot.

```rust
// Less safe: the program counter and a step map key are indistinguishable u32s.
pub struct Context {
    pub pc: u32,
    pub steps: HashMap<u32, StepState>,
}
fn set_value(ctx: &mut Context, var: &str, value: Value);

// Better: newtypes make the roles unambiguous.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pc(pub u32);

pub struct Context {
    pub pc: Pc,
    pub steps: HashMap<Pc, StepState>,
}
```

Avoid return tuples with several unrelated values; return a named enum or struct instead.

```rust
// Incorrect
fn step(ir: &[Instr], ctx: &mut Context) -> Result<(Option<String>, bool, EvalError?), ()>;

// Correct
pub fn step(ir: &[Instr], ctx: &mut Context) -> StepOutcome;
// where StepOutcome = Instruction(String) | Complete | NotRunning | Error(EvalError)
```

## 4.2 Use enums for mutually exclusive state

Do not encode a state machine with several booleans; model mutually exclusive states as an enum.

```rust
// Incorrect: invalid combinations are possible.
struct RunState {
    is_running: bool,
    is_complete: bool,
    is_halted: bool,
}

// Correct
pub enum Status {
    Running,
    Complete,
    Halted(String),
}

// Correct: the outcome of an operation, with the reason attached where relevant.
pub enum CheckOutcome {
    Advanced,
    Pending,
    Failed,
    Done,
    NotRunning,
    Error(EvalError),
}
```

## 4.3 Trait design

Traits SHOULD express a single, stable capability boundary; do not bundle unrelated
responsibilities.

```rust
// Incorrect: one trait mixes parsing, execution, persistence, and rendering.
trait AppManager {
    fn parse(&self, source: &str) -> Module;
    fn step(&self, ir: &[Instr], ctx: &mut Context) -> StepOutcome;
    fn save(&self, dir: &Path, ctx: &Context);
    fn render(&self) -> String;
}

// Better: one boundary per capability.
trait InstanceStore {
    fn start(&self, workflow_src: &str) -> Result<(), InstanceError>;
    fn load_context(&self) -> Result<Context, InstanceError>;
    fn save_context(&self, ctx: &Context) -> Result<(), InstanceError>;
}
```

Use generics or `impl Trait` when the concrete type is known at compile time and callers benefit
from static dispatch. Use `dyn Trait` only when runtime-selectable implementations are genuinely
needed.

## 4.4 `Option` and `Result`

Use `Option<T>` only when absence is expected and needs no explanation.

```rust
// Correct: a step's verification result is genuinely optional.
pub struct StepState {
    pub checked: Option<bool>,
    pub attempts: u32,
}

// Correct: looking up a step by pc may miss.
fn step_state(ctx: &Context, pc: u32) -> Option<&StepState> {
    ctx.steps.get(&pc)
}
```

Use `Result<T, E>` when the caller needs to know why an operation failed.

```rust
// Incorrect: malformed source is not ordinary absence.
fn parse(source: &str) -> Option<Module>;

// Correct
fn parse(source: &str) -> Result<Module, ParseError>;
fn eval(expr: &Expr, vars: &HashMap<String, Value>) -> Result<Value, EvalError>;
```

Public fallible APIs SHOULD return a meaningful, domain-specific error type, never a bare
`String`.

```rust
// Incorrect
fn load_context(dir: &Path) -> Result<Context, String>;

// Correct
fn load_context(dir: &Path) -> Result<Context, InstanceError>;
```
