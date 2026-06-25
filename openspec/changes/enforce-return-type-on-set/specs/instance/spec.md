## MODIFIED Requirements

### Requirement: set writes typed values

When the user runs `steer instance set <name> <var> <value>`, JSON literals
are parsed as typed values, and bare strings remain strings. When `<var>` is
the current op's assignment target, `set` additionally enforces the callee's
declared `return` `ParamKind`: a value whose type does not match is rejected
with a reason and is **not stored**. When `<var>` is not the current op's
target, `set` accepts the parsed value without type checking. The special
`checked` variable keeps its own structural validation
(`true`/`false` or `{"passed":bool,"reason":"..."}`) and is unaffected by
return-type enforcement.

#### Scenario: typed parsing is unchanged

- **WHEN** the user runs `steer instance set <name> <var> <value>` for a
  variable that is not the current op's target
- **THEN** JSON literals are parsed as typed values, and bare strings remain
  strings, with no type checking.

#### Scenario: a wrong-typed value for the current op is rejected at set

- **WHEN** the agent sets the current op's target variable to a value whose
  type does not match the callee's declared `return` kind
- **THEN** `set` exits non-zero with a reason and does not store the value.

#### Scenario: checked variable keeps its structural validation

- **WHEN** the user runs `steer instance set <name> checked <value>`
- **THEN** the existing `checked` validation applies (`true`/`false` or
  `{"passed":bool,"reason":"..."}`) and is not affected by return-type
  enforcement.
