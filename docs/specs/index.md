# Behavior Specs

> Living behavior specification for the current tool. These scenarios describe
> implemented semantics and regression expectations, organized by
> implementation layer. Browse the topic that matches your task.

| Topic | Description |
|-------|-------------|
| [CLI Surface](./cli.md) | Version output and subcommand structure. |
| [Lexing](./lexing.md) | Tokenization, string interpolation, comments, spans. |
| [Parsing And AST](./parsing.md) | Statement forms, control structures, precedence, reserved words. |
| [Validation](./validation.md) | Semantic checks: `return`, `produce`, reserved names, function placement, error reporting. |
| [Workflow File Discovery](./discovery.md) | Resolving the `<workflow>` argument: path precedence and `.steer/workflows/` fallback. |
| [Workflow Listing](./workflow-listing.md) | `steer workflow list`: enumerating workflows and their `@description` directive. |
| [Templates And Instruction Rendering](./templates.md) | Jinja2 subset, `@template` selection, fallback order, target-aware return prompts. |
| [Runtime Check Flow](./runtime-check.md) | The `step`/`check` cycle, the `checked` flag, retry with failure reason. |
| [Simulation](./simulation.md) | Static dry-run rendering of every action node. |
| [IR And VM Semantics](./ir-vm.md) | Lowering AST to instructions, VM execution, JSON round-trip. |
| [Loop And Branch Conditions](./control-flow.md) | Steer-side predicate purity, post-test loops, `judge` vs `check`. |
| [Instance Lifecycle](./instance.md) | `start`, name validation, typed `set`, fatal `error`, resume. |
