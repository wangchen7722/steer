# 8. Async and Concurrency

> Part of the **Rust Code Style**. See [`index.md`](./index.md) for the scope and the meaning of
> **MUST / SHOULD / MAY**.

> **When this applies:** these rules matter once the codebase introduces `async`/`await`, threads,
> locks, or `static` mutable state. A fully synchronous codebase does not exercise them — keep a
> pure, blocking design where practical — but agree the conventions now so they are settled before
> async or shared state is added.

## 8.1 Do not hold locks across `.await`

A mutex guard MUST NOT remain live across `.await` unless the lock type and the API are explicitly
designed for that usage.

```rust
// Incorrect
let mut state = self.state.lock().await;
state.steps.insert(pc, StepState::default());

do_async_work().await?;

// Correct: drop the guard before awaiting.
{
    let mut state = self.state.lock().await;
    state.steps.insert(pc, StepState::default());
}

let result = do_async_work().await?;
```

## 8.2 Cancellation and task ownership

Async operations SHOULD make cancellation and ownership boundaries explicit.

```rust
// Incorrect: it is unclear who owns and cancels the spawned task.
tokio::spawn(run_instance(name));

// Better: retain the handle or document deliberate detachment.
let handle = tokio::spawn(run_instance(name));
self.running.insert(name, handle);
```

Do not spawn background tasks merely to avoid defining ownership and completion semantics.

## 8.3 Avoid shared mutable state by default

Prefer passing a `&mut Context` through the call chain over any global or shared mutable state.

```rust
// Less clear: one global mutable run state.
static CONTEXT: LazyLock<Mutex<Context>> = /* ... */;

// Better: the context belongs to one run and is passed by reference.
pub fn step(ir: &[Instr], ctx: &mut Context) -> StepOutcome {
    // ...
}
```
