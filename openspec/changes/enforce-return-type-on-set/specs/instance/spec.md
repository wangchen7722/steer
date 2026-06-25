## MODIFIED Requirements

### Requirement: set writes typed values

When the user runs `steer instance set <name> <var> <value>`, JSON literals
are parsed as typed values, and bare strings remain strings. `set` itself
performs no return-type validation — a wrong-typed value is stored as parsed.
The value's type is enforced later, at `check` time, against the callee's
declared `return` `ParamKind`: a mismatch makes `check` return `Failed` with a
reason (a recoverable retry condition), not a silent success. The special
`checked` variable keeps its own structural validation (`true`/`false` or
`{"passed":bool,"reason":"..."}`) and is unaffected.

#### Scenario: typed parsing is unchanged

- **WHEN** the user runs `steer instance set <name> <var> <value>`
- **THEN** JSON literals are parsed as typed values, and bare strings remain
  strings.

#### Scenario: a wrong-typed value is stored but later rejected at check

- **WHEN** the agent sets a variable to a value whose type does not match the
  callee's declared `return` kind
- **THEN** `set` accepts and stores it, but the subsequent `check` on that op
  returns `Failed` with a reason instead of advancing.

#### Scenario: checked variable keeps its structural validation

- **WHEN** the user runs `steer instance set <name> checked <value>`
- **THEN** the existing `checked` validation applies (`true`/`false` or
  `{"passed":bool,"reason":"..."}`) and is not affected by return-type
  enforcement.
