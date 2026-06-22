# steer

A **control unit for agent-driven dynamic workflows**. steer interprets a
workflow written as code + natural language, advances it step by step, renders
each action as an instruction for an external agent to execute, and writes the
agent's reported results back into the execution context.

steer itself only runs control flow (assignment, branching, loops, functions),
renders instructions, and manages state — the program counter, variables, call
stack, and per-step verification results, all serialised to disk. It does **not**
run shell commands, touch the filesystem, spawn processes, or touch the
terminal. Every interaction with the outside world is rendered into an
instruction for the agent, and results flow back via `steer instance set`.

> Design and rationale: [`docs/design.md`](./docs/design.md). Behavior specs:
> [`docs/specs/`](./docs/specs/index.md).

## What steer provides

- **Language** (`steer-syntax`): a lexer, parser, and AST for the `.steer` DSL —
  assignments, `if/elseif/else`, `loop…until`, `for x in list`, `func/return`,
  calls with positional + named arguments, strings with `{var}` interpolation,
  arithmetic, comparison, and `and`/`or`/`not` logical operators.
- **Authoring** (`steer-core` + CLI): `steer workflow validate` (syntax +
  semantic checks) and `steer workflow simulate` (render every instruction).
- **Runtime** (`steer-core` + CLI): a flat IR, a stepping interpreter, a
  serialisable execution context, and `steer instance start/step/check/set/
  error/status` — the agent-driven loop, resumable across CLI calls via
  `.steer/instances/<name>/context.json`.
- **Templates**: a minimal Jinja2-subset renderer with built-in
  `task`/`ask`/`command`/`collect`/`judge`/`print` templates, plus
  file-based `.steer/templates/<dir>/<node>.j2.md` overrides selected by
  `@template`.

## Build

```bash
cargo build --release      # binary at target/release/steer
cargo test                 # ~130 tests across the workspace
cargo clippy --all-targets -- -D warnings
```

## The `.steer` language

```
// assignments, calls, comments
x = 5
toolchain = ask("which build system?", return="a string")
print("hi {toolchain}")

// control flow (if / elseif / else; explicit end / until)
if x > 3
    print("big")
elseif x > 0
    print("small")
else
    print("zero")
end

// loop until a condition; a boolean judgment comes from `judge`
i = 0
loop
    i = i + 1
    passed = judge("is the build green?")
until passed or i >= 3

for f in files
    task("fix {f}", check="confirm {f} is fixed")
end

// functions
func analyze(bug)
    existing = command("test -f root-{bug}.md", return="yes or no")
    if existing == "yes"
        return "root-{bug}.md"
    end
    task("find the root cause", return="file path", produce=["root-{bug}.md"],
         check="confirm it states the root cause")
    return "root-{bug}.md"
end
```

Action nodes (`task`, `ask`, `command`, `collect`, `judge`, `print`) are agent
operations; their arguments (`return`, `check`, `produce`) tell steer
how to render the instruction and how to verify it. `judge` is a boolean node:
it returns `true`/`false` and needs no `return=`.

## CLI

```
steer workflow validate <wf>      # check a workflow
steer workflow simulate <wf>      # print every instruction it emits

steer instance start <wf> <name>  # create / reset an instance
steer instance step <name>        # current instruction (no state change)
steer instance check <name>       # advance past the current op
steer instance set <name> <v> <val>   # report a value or `checked`
steer instance error <name> "<reason>"   # halt
steer instance status <name>      # running / complete / halted
```

The `<wf>` argument for `instance start`, `workflow validate`, and
`workflow simulate` is resolved as: the path as given first; if that is not a
file, a flat lookup under `.steer/workflows/` by name, auto-appending `.steer`
when the extension is missing. So `steer instance start bugfix-loop myrun`
loads `.steer/workflows/bugfix-loop.steer`.

## Layout

```
crates/steer-syntax   lexer, parser, AST (LSP-friendly, no I/O)
crates/steer-core     validation, templates, IR, VM, instance storage
crates/steer-cli      the `steer` binary
.steer/               shipped workflows, templates, and instance storage
docs/                 design rationale and behavior specs
.claude/skills/       the agent skills that author and drive workflows
```

## License

MIT.
