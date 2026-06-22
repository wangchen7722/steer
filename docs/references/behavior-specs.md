# Steer Behavior Specs

> Living behavior specification for the current tool. These scenarios describe
> implemented semantics and regression expectations for the DSL, compiler,
> runtime, templates, CLI, and examples.

## CLI Surface

### Scenario: print version
- **WHEN** the user runs `steer --version`
- **THEN** the binary prints a version string and exits successfully.

### Scenario: subcommands are recognized
- **WHEN** the user runs `steer workflow {validate,simulate}` or
  `steer instance {start,status,step,check,set,error}`
- **THEN** the CLI parses the subcommand and its positional arguments.

## Lexing

### Scenario: tokenize an assignment
- **WHEN** the source is `x = 5`
- **THEN** the lexer emits `Ident("x"), Assign, Int(5), Newline, Eof`.

### Scenario: string with interpolation
- **WHEN** the source is `"hello {name}!"`
- **THEN** the lexer emits one `Token::String(Vec<Spanned<StringSegment>>)`
  token whose segments are `Literal("hello ")`, `Interpolation("name")`, and
  `Literal("!")`.

### Scenario: interpolation spans are source-global
- **WHEN** a string interpolation is parsed
- **THEN** the inner expression span maps back to the original source byte
  offsets, not to an interpolation-local `0..N` range.

### Scenario: line comments are ignored and multi-line calls are supported
- **WHEN** a line has a trailing `// comment`, or a call spans lines inside
  parentheses, brackets, or braces
- **THEN** comments produce no tokens and inner newlines are suppressed.

### Scenario: unclosed delimiters do not fabricate a top-level newline
- **WHEN** EOF is reached while delimiter depth is non-zero
- **THEN** the lexer does not emit a synthetic top-level `Newline`; the parser
  reports the missing delimiter.

### Scenario: raw newlines in strings are rejected
- **WHEN** a string literal crosses a physical line
- **THEN** lexing fails with `NewlineInString`; authors must use the `\n`
  escape.

### Scenario: interpolation bodies are restricted
- **WHEN** an interpolation body contains a raw `"` or nested `{`
- **THEN** lexing rejects it instead of silently mis-segmenting the string.

### Scenario: spans are byte offsets
- **WHEN** tokenizing `ab = 1`
- **THEN** `Ident("ab")` spans bytes `0..2` and `Int(1)` spans bytes `5..6`.

## Parsing And AST

### Scenario: public AST names match the implemented model
- **WHEN** consumers inspect the syntax AST
- **THEN** statements use `Stmt::Call` for standalone calls, expressions use
  `Expr::String`, string pieces use `StringPart::{Literal,Interpolation}`, call
  arguments use `CallArg::{Positional,Named}`, and binary operators use
  `BinaryOp`.

### Scenario: standalone non-call expressions are rejected
- **WHEN** a workflow contains a bare expression statement such as `1 + 2`,
  `"hello"`, or `x == y`
- **THEN** parsing fails because a standalone statement must be a call.

### Scenario: assignment and calls
- **WHEN** the source is `x = 5`, `task("do")`, or
  `result = task("do", return="path")`
- **THEN** parsing produces the corresponding `Assign` or `Call` AST.

### Scenario: meta directives
- **WHEN** the source contains `@template = "planning"`
- **THEN** parsing produces `Stmt::Meta { key: "template", value: ... }`.

### Scenario: control structures parse to their AST forms
- **WHEN** the source contains `if/elseif/else/end`, `loop ... until cond`,
  `for x in list ... end`, `func ... end`, or `return expr`
- **THEN** each parses to the corresponding statement with its body block.

### Scenario: operator precedence
- **WHEN** the source is `1 + 2 * 3`
- **THEN** parsing groups it as `1 + (2 * 3)`.

### Scenario: positional argument after a named one is rejected
- **WHEN** a call lists a positional argument after a named one
- **THEN** parsing returns `ParseErrorKind::PositionalAfterNamed`.

### Scenario: reserved words are not identifiers
- **WHEN** `not`, `and`, `or`, or `in` is used as an assignment target,
  argument name, or bare variable
- **THEN** parsing rejects it.

## Validation

### Scenario: valid workflow
- **WHEN** the user runs `steer workflow validate <valid-file>`
- **THEN** the CLI prints `<path>: OK` and exits 0.

### Scenario: value node assigned without `return`
- **WHEN** a value node (`task`, `ask`, `command`, or `collect`) is assigned
  without `return=`
- **THEN** validation reports an error.

### Scenario: bare task without `return` is allowed
- **WHEN** `task(...)` is used only for side effects
- **THEN** validation reports no error.

### Scenario: `return` prompt is required only for a real assignment target
- **WHEN** a bare call includes `return=`
- **THEN** the rendered instruction does not ask the agent to run
  `steer instance set <name> <var>` because no variable is receiving the value.

### Scenario: argument type rules
- **WHEN** `produce=` is not a list literal, `check=` or `return=` is not a
  string literal, or a function/parameter/named argument is duplicated
- **THEN** validation reports an error.

### Scenario: reserved runtime names
- **WHEN** a workflow assigns to `checked` or to a name beginning with `__`
- **THEN** validation rejects the workflow.

### Scenario: functions are top-level only
- **WHEN** a `func` appears inside another statement body
- **THEN** validation rejects it.

### Scenario: parse error reports a location
- **WHEN** the workflow has a syntax error
- **THEN** the CLI prints the message with `at line L, col C` and exits
  non-zero.

## Workflow File Discovery

### Scenario: workflow path is resolved under `.steer/workflows/`
- **WHEN** the user runs `instance start`, `workflow validate`, or
  `workflow simulate` with a path that is not an existing file in the current
  directory
- **THEN** the CLI falls back to a flat lookup under `.steer/workflows/` by file
  name, so `bugfix-loop.steer` resolves to `.steer/workflows/bugfix-loop.steer`.

### Scenario: a bare name auto-appends `.steer`
- **WHEN** the user passes a name with no extension, e.g. `bugfix-loop`
- **THEN** discovery tries `.steer/workflows/bugfix-loop.steer`.

### Scenario: an explicit path takes precedence
- **WHEN** a file matching the given path exists in the current directory
- **THEN** it is read directly and the `.steer/workflows/` fallback is not
  consulted, even if a same-named file exists there.

### Scenario: nothing matches keeps the original error
- **WHEN** the path is not found at the given location and no same-named file
  exists under `.steer/workflows/`
- **THEN** the CLI reports `cannot read <original path>` as before.

## Templates And Instruction Rendering

### Scenario: Jinja-style interpolation, if, and for
- **WHEN** a template uses `{{ name }}`, `{% if %}/{% else %}/{% endif %}`, or
  `{% for x in list %}/{% endfor %}`
- **THEN** it renders against the call arguments and runtime values.

### Scenario: workflow template directory selection
- **WHEN** `@template = "planning"` executes before an action node
- **THEN** subsequent action nodes first resolve templates from
  `.steer/templates/planning/<callee>.j2.md`.

### Scenario: template fallback order
- **WHEN** an action node is rendered
- **THEN** resolution checks the active template directory, then
  `.steer/templates/default/<callee>.j2.md`, then the built-in template.

### Scenario: template selection persists across resume
- **WHEN** a workflow changes `@template` and the context is serialized
- **THEN** `context.json` preserves `meta.template_dir` and resumed execution
  uses the same active template directory.

### Scenario: step instructions do not include check mechanics
- **WHEN** `step` renders an action node with `check=`
- **THEN** the task instruction is rendered without the verification prompt;
  verification is rendered by `check`.

### Scenario: value return prompt is target-aware
- **WHEN** `x = task("...", return="...")` is rendered
- **THEN** the instruction tells the agent to set `x`.
- **WHEN** a bare `task("...", return="...")` is rendered
- **THEN** the instruction does not render a `steer instance set <name> <var>` prompt.

### Scenario: runtime interpolation is preserved for simulation
- **WHEN** simulation renders an instruction containing `{f}` for a runtime
  variable
- **THEN** the rendered instruction keeps `{f}` as a placeholder.

## Runtime Check Flow

### Scenario: step pauses at the next action node
- **WHEN** `step` is called on a running context
- **THEN** it executes control instructions and pauses at the next action node,
  returning the rendered task instruction.

### Scenario: check renders verification instruction
- **WHEN** `check` runs on an action node with `check="run tests"`
- **THEN** it returns an instruction containing `run tests` and the runtime
  appended reporting commands:
  `steer instance set <name> checked {"passed":true}` and
  `steer instance set <name> checked {"passed":false,"reason":"<why it failed>"}`.

### Scenario: failed check requires a reason
- **WHEN** the user runs `steer instance set <name> checked false`
- **THEN** the command is rejected.
- **WHEN** the user runs
  `steer instance set <name> checked '{"passed":false,"reason":"tests failed"}'`
- **THEN** `check` returns `failed` and stores the reason on the current step.

### Scenario: retry instruction includes the previous failure reason
- **WHEN** a step's previous verification failed with a reason
- **THEN** the next `step` for the same action appends that reason and asks the
  agent to address it before checking again.

### Scenario: passing check advances
- **WHEN** the user runs `steer instance set <name> checked true` or sets
  `{"passed":true}`
- **THEN** `check` advances past the current action node.

### Scenario: value-op check waits for the value
- **WHEN** `check` runs on an assigned value op before its value is set
- **THEN** it returns `Pending`; once `set` supplies the value, `check`
  advances.

### Scenario: bare op check advances immediately
- **WHEN** `check` runs on an action node with no value target and no `check`
  clause
- **THEN** it advances immediately.

## Simulation

### Scenario: render all action nodes in order
- **WHEN** the user runs `steer workflow simulate <wf>`
- **THEN** the CLI prints each action node's rendered instruction, numbered, in
  source order.

### Scenario: static walk
- **WHEN** the workflow has loops and branches
- **THEN** each action node is shown once; loops are not expanded and both
  branches of an `if` are shown.

### Scenario: user-function call sites are not action instructions
- **WHEN** a workflow calls a user function
- **THEN** the call site itself produces no instruction; action nodes in the
  function body are shown during the static walk.

### Scenario: nested action calls are rendered
- **WHEN** an action call appears inside an expression that simulation visits
- **THEN** simulation renders that action node and threads assignment targets
  into `render_call` when available.

### Scenario: empty workflow
- **WHEN** the workflow has no action nodes
- **THEN** simulate prints `(no action nodes)`.

## IR And VM Semantics

### Scenario: sequential statements
- **WHEN** a module of sequential statements is lowered
- **THEN** the instruction stream is those instructions followed by `Halt`.

### Scenario: meta directives lower to runtime instructions
- **WHEN** the AST contains `Stmt::Meta { key: "template", ... }`
- **THEN** lowering emits `Instr::SetMeta`, and VM execution updates
  `ctx.meta.template_dir`.

### Scenario: if / elseif / else
- **WHEN** a multi-branch `if` is lowered
- **THEN** each branch emits a conditional jump to the next branch or else/end,
  its body, and a jump to the shared end target.

### Scenario: loops
- **WHEN** `for x in list` or `loop ... until cond` is lowered
- **THEN** `for` compiles to init + iter + a back-edge jump; `loop ... until`
  compiles to the body plus a back-edge `JumpIfFalse`.

### Scenario: function definitions and calls
- **WHEN** a module defines and calls a function
- **THEN** function bodies are emitted after `Halt` and each call resolves to
  the function body.

### Scenario: return of a call is supported uniformly
- **WHEN** `return userfunc()` or `return task(...)` is lowered
- **THEN** the call is lowered through a temporary result and then returned.

### Scenario: eval error halts the run
- **WHEN** VM execution hits an eval error
- **THEN** the context status becomes `Halted` instead of leaving the PC stuck
  on the failing instruction.

### Scenario: context round-trips through JSON
- **WHEN** a context is serialized and deserialized
- **THEN** it is unchanged, including variables, frames, step state, and
  workflow metadata.

## Loop And Branch Conditions

### Scenario: conditions are steer-side predicates
- **WHEN** an `if cond` or `until cond` is evaluated
- **THEN** `cond` is a pure expression over context variables, not an agent op.

### Scenario: world-dependent exit conditions live in the loop body
- **WHEN** a loop should exit based on outside-world state
- **THEN** the body runs an action node that sets a context variable, and the
  `until` or `if` condition reads that variable.

### Scenario: loop-until is post-test
- **WHEN** a `loop ... until cond` runs
- **THEN** the body runs at least once, then `cond` is tested.

### Scenario: judge and check are distinct mechanisms
- **WHEN** the author needs a judgment result in a condition
- **THEN** `judge("...")` returns a boolean into a variable.
- **WHEN** the author needs verify-and-retry behavior
- **THEN** `task("...", check="...")` uses the runtime checked flow.

## Instance Lifecycle

### Scenario: start creates a fresh instance
- **WHEN** the user runs `steer instance start <wf> <name>`
- **THEN** a fresh instance is created under `.steer/instances/<name>/`,
  replacing any previous instance with the same valid name.

### Scenario: invalid instance names are rejected
- **WHEN** the instance name is empty, `.`, `..`, absolute, or contains `/` or
  `\`
- **THEN** the CLI rejects it before touching `.steer/instances`.

### Scenario: set writes typed values
- **WHEN** the user runs `steer instance set <name> <var> <value>`
- **THEN** JSON literals are parsed as typed values, and bare strings remain
  strings.

### Scenario: error halts and status reports state
- **WHEN** the user runs `steer instance error <name> <reason>` then `status`
- **THEN** the run is `Halted` and `status` reports it.

### Scenario: resume across CLI calls
- **WHEN** `step`, `check`, and `set` are issued as separate CLI invocations
- **THEN** persisted context lets the run continue from the same PC and state.

## Examples

### Scenario: shipped examples validate and simulate
- **WHEN** each workflow under `examples/workflows/` is validated and simulated
  from the `examples/` directory
- **THEN** all examples pass validation, render without errors, and cover
  bugfix retry loops, OpenSpec-style proposal/design/specs/tasks flow, and
  `@template` switching.

### Scenario: temporary smoke workflow is not part of the examples
- **WHEN** the repository is inspected
- **THEN** `.steer/workflows/smoke-bugfix.steer` is absent because it was a
  temporary development workflow, replaced by realistic examples.
