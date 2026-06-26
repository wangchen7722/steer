# Steer CLI Command Reference

## Workflow Commands

### `steer workflow validate <workflow>`

Syntax + semantic check. Reports errors with line/column.

```bash
steer workflow validate .steer/workflows/my-workflow.steer
# or by name (auto-resolves under .steer/workflows/):
steer workflow validate my-workflow
```

Semantic checks include:
- `return` required for assigned value ops (`ask`, `command`, `collect`)
- `produce` must be a list
- No reserved words used as variable names
- Functions are top-level only

### `steer workflow simulate <workflow>`

Dry-run: renders every action node once and prints the output. Useful for
previewing what the agent will see without running the workflow.

```bash
steer workflow simulate my-workflow
```

Both branches of `if` and all function bodies are rendered.

## Instance Commands

### `steer instance start <workflow> <name>`

Create (or reset) an instance and initialise its program counter.

```bash
steer instance start my-workflow myrun
```

Output:
```
instance myrun: started
```

If `@context` is set in the workflow, the context description is appended:
```
instance myrun: started
bug-fix workflow for login issues
```

Starting with an existing name clears and recreates the instance.

### `steer instance step <name>`

Return the instruction at the current program counter, without changing state.

```bash
steer instance step myrun
```

Possible outputs:
- Rendered instruction (XML block)
- `(complete)`: workflow finished
- `(not running)`: instance is halted or corrupted

If the previous check failed, the instruction includes retry context.

### `steer instance check <name>`

Advance the program counter, dispatching by node type.

```bash
steer instance check myrun
```

Possible results:
| Output | Meaning |
|--------|---------|
| `advanced` | Op passed; steer advanced to the next step |
| `pending` | Value not reported yet; `set` it first |
| (instruction) | Check instruction for the agent to verify |
| `failed` | Check failed; retry the same instruction |
| `(done)` | Workflow already complete |
| `(not running)` | Instance halted |

### `steer instance set <name> <var> <value>`

Write a typed value into the instance context.

```bash
# String:
steer instance set myrun files "src/main.rs"

# Int:
steer instance set myrun count 42

# Bool:
steer instance set myrun passed true

# List:
steer instance set myrun items "[1,2,3]"

# Checked (pass):
steer instance set myrun checked true
steer instance set myrun checked '{"passed":true}'

# Checked (fail, reason required):
steer instance set myrun checked '{"passed":false,"reason":"test still failing"}'
```

Value types: `[1,2,3]` (list), `42` (int), `3.14` (float), `true`/`false`
(bool), `"text"` (string), bare word (string).

**Shell quoting note:** JSON values with `{` and `}` must be single-quoted in
bash to prevent brace expansion:

```bash
steer instance set myrun checked '{"passed":true}'     # correct
steer instance set myrun checked {"passed":true}        # ERROR: bash interprets { }
```

### `steer instance error <name> "<reason>"`

Report a fatal failure; the instance halts immediately.

```bash
steer instance error myrun "cannot reproduce the bug"
```

Status becomes `halted: cannot reproduce the bug`.

### `steer instance status <name>`

Show the instance status.

```bash
steer instance status myrun
```

Output:
```
instance myrun: running
bug-fix workflow for login issues
```

Possible statuses: `running`, `complete`, `halted: <reason>`.

The output includes the workflow context (if `@context` was set).

## Workflow Path Resolution

The `<workflow>` argument is resolved in this order:

1. The path as given (if it exists as a file)
2. Flat lookup under `.steer/workflows/` by name
3. Auto-append `.steer` extension if the name has no extension

So all of these work:

```bash
steer instance start .steer/workflows/bugfix-loop.steer myrun
steer instance start .steer/workflows/bugfix-loop myrun
steer instance start bugfix-loop myrun
```

## Agent-Driven Loop

The agent (running the steer skill) drives the execution cycle:

```
step → execute → [set value] → check → repeat
```

1. `step` to get the current instruction.
2. Execute the instruction.
3. If the instruction has a `<report>` tag, `set` the value.
4. If the instruction is a check, perform verification, `set checked`, then `check`.
5. `check` to advance.
6. Repeat until `(complete)` or `(not running)`.
