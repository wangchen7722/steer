# Steer DSL Syntax Reference

## File Convention

- Workflow files end in `.steer` and live under `.steer/workflows/`.
- Lines starting with `//` are comments (until end of line).
- Names are identifiers: letters, digits, underscores. Reserved words (`and`, `or`, `not`, `in`) cannot be used as variable names or named-argument names.

## Statements

### Assignment

```steer
x = 5
toolchain = ask("which build system?", return="a string")
files = ["src/main.rs", "Cargo.toml"]
```

### Meta Directives (`@`)

```steer
@template = "planning"    # switch active template directory
@context = "description"   # set workflow-level context description
```

- `@template` changes where subsequent action nodes resolve their `.j2.md` templates.
- `@context` sets a description shown on `instance start` and `instance status`.
- Both persist across resume. Set `@template = "default"` to revert.

### Standalone Call (Side-effect Node)

```steer
print("done")
task("do something")
```

## Control Flow

### `if` / `elseif` / `else`

```steer
if x > 3
    print("big")
elseif x > 0
    print("small")
else
    print("zero")
end
```

- `elseif` is one word (no space).
- Each branch body is a block of statements.
- `end` closes the `if`.

### `loop ... until`

Post-test loop: body runs at least once.

```steer
i = 0
passed = false
loop
    i = i + 1
    task("try fix", check="verify the fix")
    passed = judge("did the fix work?")
until passed or i >= 3
```

- Condition is a boolean expression.
- No separate counted-loop form; use a counter variable + `until`.

### `for x in list`

Iterate over list elements.

```steer
for f in files
    task("review {f}", check="confirm {f} is clean")
end
```

- `for` iterates list elements only (does not split strings).
- The list is evaluated once at entry.

### `func` / `return`

User-defined functions (top-level only, cannot nest).

```steer
func analyze(bug)
    existing = command("test -f root-{bug}.md", return="yes or no")
    if existing == "yes"
        return "root-{bug}.md"
    end
    task("find the root cause", return="file path",
         produce=["root-{bug}.md"], check="confirm it states the root cause")
    return "root-{bug}.md"
end
```

- Functions are compiled after the main program (reachable via `Call`).
- `return` without a value is valid.
- A top-level `return` halts the entire workflow.

## Expressions

### Literals

| Type | Example |
|------|---------|
| Int | `42`, `0`, `-7` |
| Float | `3.14`, `-0.5` |
| String | `"hello"`, `"with {var} interpolation"` |
| List | `[1, 2, 3]`, `[]`, `["a", "b"]` |
| Bool | `true`, `false` |
| Null | `null` |

### String Interpolation

```steer
"hello {name}!"
"attempt {attempt} of {max}"
```

- Interpolation bodies contain a single expression.
- Escapes: `\n`, `\t`, `\"`, `\\`, `\{`, `\}`.
- No nested `"` or `{` inside interpolation bodies.

### Operators

| Precedence (low→high) | Operators |
|------------------------|-----------|
| 1 | `or` (short-circuit) |
| 2 | `and` (short-circuit) |
| 3 | `not` (binds looser than comparisons) |
| 4 | `==`, `!=`, `<`, `>`, `<=`, `>=` |
| 5 | `+`, `-` |
| 6 | `*`, `/` |
| 7 | `-` (unary negation) |

- `not a == b` means `not (a == b)`.
- Division between two integers returns a float.

### Truthiness

| Value | Truthy? |
|-------|---------|
| `null` | No |
| `false` | No |
| `""` (empty string) | No |
| `[]` (empty list) | No |
| `{}` (empty object) | No |
| Everything else | Yes |

(including `0`, `0.0`, `"0"`, `"false"`)

### Important Constraint

Call expressions **cannot** appear as sub-expressions within conditions or other
expressions. They must be standalone statements. This is by design: calls
interact with the external agent and must be sequenced explicitly.

```steer
// VALID:
result = command("git status", return="output")

// INVALID:
if command("git status", return="output") == "clean"  // ERROR
    ...
end
```

Use a two-step pattern instead: assign the result, then test the variable.

## Action Nodes

### `task`: Universal Agent Primitive

```steer
task("instruction text")
task("instruction text", return="expected value format")
task("instruction text", check="verification instruction")
task("instruction text", produce=["file1.md", "file2.rs"])
```

The agent does work (code, edits, writing). Most commonly used node.

### `ask`: Obtain Value from Human User

```steer
bug = ask("Which bug to fix?", return="bug identifier")
```

- `return` is **required**.
- Agent must use `AskUserQuestion` tool; never answer on the user's behalf.

### `command`: Run Shell Command

```steer
files = command("git diff --name-only", return="list of file paths")
```

- `return` defaults to `"output"`.
- Agent runs the command and captures stdout/stderr/exit code.

### `collect`: Agent Investigates and Reports

```steer
root_cause = collect("Reproduce and summarize the root cause",
                     return="root cause summary",
                     check="confirm the summary names the failing path")
```

- `return` defaults to `"result"`.
- Agent must do actual work (read files, trace behavior), not guess.

### `judge`: Boolean Judgment

```steer
passed = judge("Did the test pass?")
```

- Returns `true` or `false` (intrinsic boolean).
- No `return=` argument.
- Used in conditions: `until passed or i >= 3`.

### `print`: Output to User

```steer
print("workflow complete for {bug}")
```

- No value produced. No `return`.
- Side-effect only.

## Call Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| (positional) | string | Yes | The instruction text (first positional arg) |
| `return` | string | Varies | Expected format of the result |
| `check` | string | No | Verification instruction (enables checked flow) |
| `produce` | list | No | Files the step should create |
| `spawn` | bool | No | Whether to use a sub-agent |

- `return` is required for `ask`, optional for `task`, defaults for `command` and `collect`.
- `judge` has no `return` argument (intrinsically boolean).
- `print` has no `return`.
