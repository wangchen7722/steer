# OpenSpec Generate Specs Workflow

## Purpose

The `openspec-generate-specs` `.steer` workflow generates or refreshes OpenSpec
main specs from existing code, docs, and full git history, fully automatically
(no mid-run ask points), with a closed review-and-refine coverage loop and an
audit dossier at `.openspec-generate-runs/<run>/`.

## Requirements

### Requirement: A startup gate confirms the openspec CLI and the run slug is the sole input

The workflow SHALL gate startup on `command -v openspec` (rendered via a
`command(...)` call returning `yes` or `no`); if the binary is absent it SHALL
`print` a STOP message and `return` before doing any work, because
`openspec validate` is the run's sole validation dependency. It SHALL ask exactly
one input variable — `run` — with a kebab-case `return=` type; from that point on
the run is fully automatic with no mid-run ask points. The
[`runtime-check-gate`](openspec/specs/runtime-check-gate/spec.md) engine
capability backs the `command(...)` check.

#### Scenario: a missing openspec CLI halts the run

- **WHEN** `command -v openspec` does not find the binary
- **THEN** the workflow prints a STOP message and returns before the run slug is
  even asked for.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the
  `has_openspec`/`if`/`return` block.

#### Scenario: the run slug is the only input

- **WHEN** the startup gate passes
- **THEN** the workflow asks the `run` slug once with a kebab-case `return=` type
  and proceeds fully automatically with no further ask points.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the
  `run = ask(...)` call.

### Requirement: The audit dossier lives under .openspec-generate-runs and never the manifest root

The workflow SHALL write every run artifact under
`.openspec-generate-runs/<run>/`. In a `repo`-manifest checkout where the project
root is not itself a git repo, it SHALL NEVER write `openspec/` to the manifest
root; instead each participating repo receives its own
`<repo>/openspec/specs/<cap>/spec.md`.

#### Scenario: run artifacts land in the run dossier

- **WHEN** any phase writes a run-level artifact (env.md, scan-plan.md,
  capabilities/, final-list.md, generation-log.md, coverage.md, review.md,
  coverage-passed/, summary.md)
- **THEN** the artifact is written under `.openspec-generate-runs/{run}/`, never
  under the manifest root.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the `produce=`
  paths in every phase call.

#### Scenario: per-repo specs are written into each repo's own openspec tree

- **WHEN** a spec is generated for a participating repo
- **THEN** it is written to that repo's own `openspec/specs/<cap>/spec.md`, never
  to the manifest root.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the `@context`
  multi-repo cwd rule.

### Requirement: The phase sequence runs detect_env through summary

The workflow SHALL run the phases in source order — `detect_env`, `plan_scan`,
`gather`, `identify`, `generate_all`, a conditional `coverage_guard`, the
review/refine loop, `full_validate`, and `summary` — where each phase is a custom
callee with a `produce=` path list and a `check=` condition. The per-phase output
contract SHALL be: `detect_env` writes `env.md` (per-target-repo inventory with a
coverage floor); `plan_scan` writes `scan-plan.md` (deep-scan units partitioned
by code semantics, no gaps or double-counts, cross-repo seams recorded);
`gather` writes `capabilities/` (one shard per repo/module); `identify` writes
`final-list.md` (one primary-owner repo plus participating repos per capability,
kebab-case names, prior capabilities marked REUSE-existing under their own repo);
`generate_all` writes one behavior-contract `<cap>/spec.md` per participating
repo and `generation-log.md` with one row per `(capability, repo)` pair;
`full_validate` runs the only whole-repo `openspec validate --specs`; `summary`
writes `summary.md` with a per-`(capability, repo)` table whose Repo column is
mandatory.

#### Scenario: each phase writes its declared dossier artifact

- **WHEN** a phase runs
- **THEN** it persists its artifact under `.openspec-generate-runs/{run}/` at the
  path named in its `produce=` list and the next phase reads that artifact.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the ten phase
  calls and their `produce=`/`check=` text.

#### Scenario: generation-log rows cover every capability-repo pair

- **WHEN** the `generate_all` phase runs
- **THEN** `generation-log.md` lists every `(capability, participating repo)` pair
  from `final-list.md` with its spec path, owner-or-participant flag, requirement
  and scenario counts, New/Merged status, and an in-place `openspec validate` exit
  0.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the
  `generate_all(...)` phase call and its `produce=`/`check=` text.

### Requirement: The coverage guard runs only for repos with prior specs and reproduces the coverage floor

The workflow SHALL branch the `coverage_guard` phase on a `collect(...)` call that
returns exactly `PRIOR` (at least one target repo had an existing
`openspec/specs/` directory — a refresh) or `BOOTSTRAP` (no target repo had prior
specs). When the token is `PRIOR`, the coverage guard SHALL confirm, per target
repo that had prior specs, that every existing capability and every
requirement/scenario/behavior contract is reproduced under the same name in that
repo (the coverage floor), fixing and re-validating any gap in place, and record
the aggregate verdict to `coverage.md`. When the token is `BOOTSTRAP`, the
coverage guard is skipped.

#### Scenario: a refresh runs the coverage guard per prior-specs repo

- **WHEN** `collect` returns `PRIOR`
- **THEN** the coverage guard runs per target repo that had prior specs and
  confirms every existing capability and requirement/scenario/behavior contract
  was reproduced under the same name.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the
  `had_prior_specs = collect(...)`, `if had_prior_specs == "PRIOR"`, and
  `coverage_guard(...)` block.

#### Scenario: a pure bootstrap skips the coverage guard

- **WHEN** `collect` returns `BOOTSTRAP`
- **THEN** the coverage guard phase is skipped entirely.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the
  `had_prior_specs = collect(...)` and `if had_prior_specs == "PRIOR"` guard.

### Requirement: The review-and-refine loop is bounded by max_review_rounds with passed-units logging

The workflow SHALL run a closed review-and-refine loop as a `loop ... until` with
`round` starting at 0, incrementing once per iteration, and `covered` starting at
`false`. Each iteration SHALL run `review` (scanning behavior-bearing evidence
units across every target repo, recording PASS units to a per-shard passed-units
log so later rounds skip them and re-check only prior gaps and refine-edited
records), then set `covered = judge(...)` true only when every target repo's
behavior GAP list is empty, and when not covered run `refine` (adding or merging
behavior-level requirements into the behavior-owning repo and re-validating in
place). The loop SHALL terminate when `covered` is true or `round >= max_review_rounds`,
where `max_review_rounds = 3` is a tunable budget declared at the top of the
program. The [`runtime-check-gate`](openspec/specs/runtime-check-gate/spec.md)
engine capability backs the per-round `judge`.

#### Scenario: the loop stops as soon as coverage is achieved

- **WHEN** a review round's `judge` sets `covered = true`
- **THEN** the `until` condition is satisfied and the loop terminates before
  `round` reaches `max_review_rounds`.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the
  `round`/`covered`/`loop`/`until` block.

#### Scenario: the loop stops at the round budget even if gaps remain

- **WHEN** `round` reaches `max_review_rounds` (3) and `covered` is still false
- **THEN** the loop terminates by budget and the workflow proceeds to
  `full_validate`.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the
  `until covered or round >= max_review_rounds` condition and `max_review_rounds = 3`.

#### Scenario: passed units are recorded and skipped in later rounds

- **WHEN** a behavior unit is recorded PASS in one review round
- **THEN** subsequent review rounds skip that unit (reading the per-shard
  passed-units log) and re-check only prior gaps and refine-edited records.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the `review(...)`
  phase call and its passed-units log `produce=` path.

### Requirement: The summary reports the lagged-covered outcome and residual gaps honestly

The `summary` phase SHALL report the `covered` outcome honestly, accounting for
the fact that `refine` runs after the last review when `covered` is false and is
not re-reviewed, so the `covered` flag MAY lag the last refine. When `covered` is
false, the summary SHALL report the residual per-repo behavior gap list from the
last review round for the user to verify whether refine closed it. The final
`print` SHALL state the `covered` outcome, the round count, that specs were
written to each repo's own `openspec/specs/` (never the manifest root), and point
at `summary.md`.

#### Scenario: a covered run reports success

- **WHEN** the loop exited with `covered = true`
- **THEN** `summary.md` reports `covered=true` and the final print states
  `covered=true` with the round count.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the
  `summary(...)` `check=` and the final `print(...)`.

#### Scenario: a budget-exhausted run reports residual gaps

- **WHEN** the loop exited by budget with `covered = false`
- **THEN** `summary.md` reports `covered=false` plus the residual per-repo
  behavior gap list from the last review round, and the final print states
  `covered=false` with the round count and points the user at the gaps.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the
  `summary(...)` `check=` (residual-gap reporting) and the final `print(...)`.

### Requirement: The multi-repo cwd rule and cross-repo link format are contract surfaces

The workflow's `@context` SHALL fix two contract surfaces for multi-repo
checkouts: validation SHALL be performed per repo via
`cd <repo> && openspec validate ...` (never at the manifest root), and
cross-repo related-spec links SHALL use the format
`[<cap> in <repo>](<repo>/openspec/specs/<cap>/spec.md)` while same-repo links
use `[<cap>](openspec/specs/<cap>/spec.md)`.

#### Scenario: validation runs per repo, not at the manifest root

- **WHEN** a spec is validated during the run
- **THEN** the validation command is `cd <repo> && openspec validate ...` executed
  inside the participating repo, not at the manifest root.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the `@context`
  directive.

#### Scenario: cross-repo links use the cross-repo format

- **WHEN** a spec references a related capability in a different repo
- **THEN** the inline link uses the `[<cap> in <repo>](<repo>/openspec/specs/<cap>/spec.md)`
  format, distinct from the same-repo `[<cap>](openspec/specs/<cap>/spec.md)`
  format.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the `@context`
  directive.

### Requirement: Scenarios describe externally observable behavior, not private implementation state

A generated `#### Scenario:` SHALL describe behavior at the black-box boundary:
its **WHEN** step names an externally observable event, input, or condition
(a request arrives, a resource is absent) by its contract surface (a message
ID, an error code), and its **THEN** step names an externally observable
consequence (a CID is allocated, an error surfaces) — never an internal field
read/write. A scenario SHALL NOT name private member variables, internal
collections, or assignment semantics in its WHEN/THEN steps. A symbol MAY
appear in a step only when it is part of an external contract (changing it
would break a cross-process, cross-compilation-unit, or cross-version
interface: message ID, error code, enum value, ABI signature, protocol field);
a process-internal symbol (a private field, an internal helper) is evidence,
not contract wording, and belongs only in the scenario's TRACE.

#### Scenario: a rejected setup surfaces as "no CID" at the caller boundary

- **WHEN** a data-call setup request is rejected
- **THEN** no new CID is allocated to the caller
- **AND** previously active CIDs remain unaffected
- **TRACE**: `vvcodec/service/src/vvcodec2.cpp` — `setupDataCall` (NOT
  `message.arg1_ = -1` / `activated_cids`; those are private encoding, not the
  contract).

#### Scenario: a private field name never appears in a step

- **WHEN** a scenario is written for a behavior whose code encodes the trigger
  in a private member (e.g. `message.arg1_ = -1`) and the outcome in an
  internal set (e.g. `activated_cids`)
- **THEN** the WHEN/THEN steps name the external event and observable
  consequence, and the private symbols appear only in the TRACE, never in the
  step wording.
- **TRACE**: this requirement is the rule itself; its evidence is the
  `vvcodec2.cpp` example above, traced at module/file/symbol granularity.

### Requirement: Spec body never carries line numbers; traceability is per-scenario TRACE at stable granularity

A generated spec SHALL NOT contain line numbers or line ranges anywhere in its
text — a line number shifts when unrelated code elsewhere in the file is
edited, so it is the least stable pointer a contract can carry. Source file
paths and symbol names are permitted (they are stable, reviewable, and
rename-aware). Every `#### Scenario:` SHALL carry a `**TRACE**:` field stating
the evidence source for that scenario's behavior at one of three granularities,
coarsest sufficient: module/subsystem, source file, or function/symbol (the
last only when finer granularity is required). The TRACE field is the spec's
traceability — the run dossier is a run-local audit log that is not committed
and is NOT the spec's long-term evidence home.

#### Scenario: a scenario traces to a stable granularity, never a line

- **WHEN** a scenario is written
- **THEN** its TRACE field names a module, source file, or function/symbol, and
  NEVER a line number or `file:line` range.
- **AND** the spec body contains no line numbers anywhere.
- **TRACE**: this requirement is the rule itself; every TRACE in this spec
  demonstrates the allowed granularities (e.g. `.steer/workflows/openspec-generate-specs.steer`).

#### Scenario: the dossier is not the spec's evidence home

- **WHEN** the run records behavior-to-evidence mappings for audit
- **THEN** those mappings live in the run dossier under
  `.openspec-generate-runs/<run>/` (which is not committed), while the spec's
  committed traceability is carried by per-scenario TRACE fields.
- **TRACE**: `.steer/workflows/openspec-generate-specs.steer` — the
  `produce=` paths that target `.openspec-generate-runs/{run}/`.
