# IR And VM Semantics

> Behavior specs for lowering (AST → instruction stream) and VM execution: sequential lowering, meta directives, multi-branch if, loops, functions, eval-error halting, and JSON round-trip.

## Scenario: sequential statements
- **WHEN** a module of sequential statements is lowered
- **THEN** the instruction stream is those instructions followed by `Halt`.

## Scenario: meta directives lower to runtime instructions
- **WHEN** the AST contains `Stmt::Meta { key: "template", ... }`
- **THEN** lowering emits `Instr::SetMeta`, and VM execution updates
  `ctx.meta.template_dir`.

## Scenario: if / elseif / else
- **WHEN** a multi-branch `if` is lowered
- **THEN** each branch emits a conditional jump to the next branch or else/end,
  its body, and a jump to the shared end target.

## Scenario: loops
- **WHEN** `for x in list` or `loop ... until cond` is lowered
- **THEN** `for` compiles to init + iter + a back-edge jump; `loop ... until`
  compiles to the body plus a back-edge `JumpIfFalse`.

## Scenario: function definitions and calls
- **WHEN** a module defines and calls a function
- **THEN** function bodies are emitted after `Halt` and each call resolves to
  the function body.

## Scenario: return of a call is supported uniformly
- **WHEN** `return userfunc()` or `return task(...)` is lowered
- **THEN** the call is lowered through a temporary result and then returned.

## Scenario: eval error halts the run
- **WHEN** VM execution hits an eval error
- **THEN** the context status becomes `Halted` instead of leaving the PC stuck
  on the failing instruction.

## Scenario: context round-trips through JSON
- **WHEN** a context is serialized and deserialized
- **THEN** it is unchanged, including variables, frames, step state, and
  workflow metadata.
