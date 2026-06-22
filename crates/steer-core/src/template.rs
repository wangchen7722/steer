//! Minimal Jinja2-subset template engine, plus the built-in action-node
//! templates and [`render_call`].
//!
//! Supported syntax: `{{ name }}` interpolation, `{% if name %}...{% else
//! %}...{% endif %}`, and `{% for x in list %}...{% endfor %}`. Whitespace
//! control (`{%-`) and expressions richer than a bare name are deliberately out
//! of scope for v1. The exact instruction wording lives in the built-in
//! templates below; steer only renders them with a call's arguments.

use std::collections::HashMap;

use steer_syntax::ast::{Call, CallArg, Expr};
use steer_syntax::Spanned;

use crate::context::WorkflowMeta;
use crate::value::{eval_literal, Value};

/// A parsed template, ready to render against a context.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
enum Node {
    Text(String),
    /// `{{ name }}`.
    Var(String),
    /// `{% if name %} then {% else %} else {% endif %}`.
    If(String, Vec<Node>, Vec<Node>),
    /// `{% for item in list %} body {% endfor %}`.
    For(String, String, Vec<Node>),
}

/// A template parsing error.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateError {
    /// An expression `{{ ... }}` was not closed.
    UnterminatedExpr,
    /// A statement tag `{% ... %}` was not closed.
    UnterminatedTag,
    /// A control block was closed by the wrong tag, e.g. `endif` for a `for`.
    WrongClose {
        expected: &'static str,
        found: String,
    },
    /// A closing tag appeared with no matching opener.
    UnmatchedClose(String),
    /// A `for` statement is malformed.
    MalformedFor(String),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::UnterminatedExpr => write!(f, "unterminated `{{{{ }}}}`"),
            TemplateError::UnterminatedTag => write!(f, "unterminated `{{% %}}`"),
            TemplateError::WrongClose { expected, found } => {
                write!(f, "expected `{expected}`, found `{found}`")
            }
            TemplateError::UnmatchedClose(t) => write!(f, "unmatched `{t}`"),
            TemplateError::MalformedFor(t) => write!(f, "malformed for statement `{t}`"),
        }
    }
}

impl std::error::Error for TemplateError {}

impl Template {
    /// Parse a template string.
    ///
    /// # Errors
    /// Returns a [`TemplateError`] if the template is malformed.
    pub fn parse(src: &str) -> Result<Self, TemplateError> {
        let pieces = lex_pieces(src)?;
        let (nodes, _next, closer) = build(&pieces, 0)?;
        if let Some(c) = closer {
            return Err(TemplateError::UnmatchedClose(c));
        }
        Ok(Template { nodes })
    }

    /// Render the template against `ctx`.
    pub fn render(&self, ctx: &HashMap<String, Value>) -> String {
        let mut out = String::new();
        render_nodes(&self.nodes, ctx, &mut out);
        out
    }
}

// ---- lexing into flat pieces ----

#[derive(Debug, Clone, PartialEq)]
enum Piece {
    Text(String),
    Expr(String),
    Tag(String),
}

fn lex_pieces(src: &str) -> Result<Vec<Piece>, TemplateError> {
    let bytes = src.as_bytes();
    let mut pieces = Vec::new();
    let mut i = 0usize;
    let mut text_start = 0usize;
    while i < bytes.len() {
        let is_expr_open = i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{';
        let is_tag_open = i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'%';
        if is_expr_open || is_tag_open {
            if i > text_start {
                pieces.push(Piece::Text(src[text_start..i].to_string()));
            }
            let close = if is_expr_open { "}}" } else { "%}" };
            let rest = &src[i + 2..];
            let end = rest.find(close).ok_or(if is_expr_open {
                TemplateError::UnterminatedExpr
            } else {
                TemplateError::UnterminatedTag
            })?;
            let inner = rest[..end].trim().to_string();
            if is_expr_open {
                pieces.push(Piece::Expr(inner));
            } else {
                pieces.push(Piece::Tag(inner));
            }
            // advance past `{{ inner }}` / `{% inner %}`
            i = i + 2 + end + close.len();
            text_start = i;
        } else {
            i += 1;
        }
    }
    if text_start < bytes.len() {
        pieces.push(Piece::Text(src[text_start..].to_string()));
    }
    Ok(pieces)
}

// ---- building the node tree ----

/// Build a sequence of nodes, stopping at a closer tag: `else`, `endif`, or
/// `endfor`. Returns `(nodes, index of the closer piece, name of the closer)`.
fn build(
    pieces: &[Piece],
    start: usize,
) -> Result<(Vec<Node>, usize, Option<String>), TemplateError> {
    let mut nodes = Vec::new();
    let mut i = start;
    while i < pieces.len() {
        match &pieces[i] {
            Piece::Text(s) => {
                nodes.push(Node::Text(s.clone()));
                i += 1;
            }
            Piece::Expr(s) => {
                nodes.push(Node::Var(s.clone()));
                i += 1;
            }
            Piece::Tag(t) => {
                let t = t.as_str();
                if t == "else" || t == "endif" || t == "endfor" {
                    return Ok((nodes, i, Some(t.to_string())));
                }
                if let Some(cond) = t.strip_prefix("if ") {
                    let (then_nodes, j, closer) = build(pieces, i + 1)?;
                    let closer = closer.ok_or(TemplateError::WrongClose {
                        expected: "endif",
                        found: "end of template".to_string(),
                    })?;
                    let mut j = j;
                    let else_nodes = if closer == "else" {
                        let (en, k, c) = build(pieces, j + 1)?;
                        if c.as_deref() != Some("endif") {
                            return Err(TemplateError::WrongClose {
                                expected: "endif",
                                found: c.unwrap_or_else(|| "end of template".into()),
                            });
                        }
                        j = k;
                        en
                    } else if closer == "endif" {
                        Vec::new()
                    } else {
                        return Err(TemplateError::WrongClose {
                            expected: "endif",
                            found: closer,
                        });
                    };
                    nodes.push(Node::If(cond.trim().to_string(), then_nodes, else_nodes));
                    i = j + 1;
                } else if let Some(rest) = t.strip_prefix("for ") {
                    let (var, list) = parse_for(rest)?;
                    let (body, j, closer) = build(pieces, i + 1)?;
                    let closer = closer.ok_or(TemplateError::WrongClose {
                        expected: "endfor",
                        found: "end of template".to_string(),
                    })?;
                    if closer != "endfor" {
                        return Err(TemplateError::WrongClose {
                            expected: "endfor",
                            found: closer,
                        });
                    }
                    nodes.push(Node::For(var, list, body));
                    i = j + 1;
                } else {
                    return Err(TemplateError::UnmatchedClose(t.to_string()));
                }
            }
        }
    }
    Ok((nodes, i, None))
}

fn parse_for(s: &str) -> Result<(String, String), TemplateError> {
    let (var, rest) = s
        .split_once(" in ")
        .ok_or_else(|| TemplateError::MalformedFor(s.to_string()))?;
    Ok((var.trim().to_string(), rest.trim().to_string()))
}

fn render_nodes(nodes: &[Node], ctx: &HashMap<String, Value>, out: &mut String) {
    for n in nodes {
        match n {
            Node::Text(s) => out.push_str(s),
            Node::Var(name) => out.push_str(&ctx.get(name).unwrap_or(&Value::Null).render()),
            Node::If(cond, then, els) => {
                if ctx.get(cond).unwrap_or(&Value::Null).truthy() {
                    render_nodes(then, ctx, out);
                } else {
                    render_nodes(els, ctx, out);
                }
            }
            Node::For(var, list, body) => {
                if let Some(Value::List(items)) = ctx.get(list) {
                    for item in items {
                        let mut sub = ctx.clone();
                        sub.insert(var.clone(), item.clone());
                        render_nodes(body, &sub, out);
                    }
                }
            }
        }
    }
}

// ---- template formatter (parameter schema) ----

/// The type of a template parameter, declared in the `formatter` section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// A string value (string literal in the DSL).
    String,
    /// A boolean — `true` or `false`.
    Bool,
    /// A list literal `[a, b, c]`.
    List,
    /// Special: marks `return` as "this node produces no value" (`return: none`).
    /// A call to such a node cannot be assigned or returned.
    None,
    /// Special: marks `return` as "intrinsic boolean" (`return: bool`). The node
    /// returns a bool without needing a `return=` argument; assignable directly.
    IntrinsicBool,
}

/// One parameter definition from a template's `formatter` section.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamSpec {
    /// The parameter name, e.g. `instruction`, `return`, `check`.
    pub name: String,
    /// The declared type.
    pub kind: ParamKind,
    /// Whether the parameter must always be present.
    pub required: bool,
    /// A declarative default value, if any.
    pub default: Option<Value>,
}

/// A parsed template definition: the `formatter` params + the `body` text.
#[derive(Debug, Clone)]
pub struct NodeTemplate {
    /// Parameter specs from the `formatter` section.
    pub params: Vec<ParamSpec>,
    /// The Jinja2 + Markdown body (the rendered instruction text).
    pub body: String,
}

impl NodeTemplate {
    /// Find the `return` parameter spec, if any.
    pub fn return_spec(&self) -> Option<&ParamSpec> {
        self.params.iter().find(|p| p.name == "return")
    }
}

/// Parse a `.j2.md` file into a [`NodeTemplate`]. If the file starts with a
/// `---` front-matter block, the text between the first two `---` lines is the
/// formatter; the rest is the body. Without front-matter, the entire file is
/// the body with a minimal formatter (a required `instruction` positional).
pub fn parse_template(src: &str) -> NodeTemplate {
    if let Some((formatter_src, body)) = split_front_matter(src) {
        NodeTemplate {
            params: parse_formatter(formatter_src),
            body: body.to_string(),
        }
    } else {
        NodeTemplate {
            params: vec![ParamSpec {
                name: "instruction".into(),
                kind: ParamKind::String,
                required: true,
                default: None,
            }],
            body: src.to_string(),
        }
    }
}

/// Split `---\n...\n---\n...` into `(formatter, body)`. Returns `None` if the
/// file does not start with `---`.
fn split_front_matter(src: &str) -> Option<(&str, &str)> {
    let after_open = src.strip_prefix("---\n")?;
    let end = after_open.find("\n---")?;
    let formatter = &after_open[..end];
    let body_start = end + "\n---".len();
    let body = after_open[body_start..]
        .strip_prefix('\n')
        .unwrap_or(&after_open[body_start..]);
    Some((formatter, body))
}

/// Parse the formatter section into [`ParamSpec`]s. Each non-empty line (after
/// an optional `formatter:` header) is `name: type[, required][, default=value]`.
fn parse_formatter(src: &str) -> Vec<ParamSpec> {
    let mut params = Vec::new();
    for line in src.lines() {
        let line = line.trim();
        if line.is_empty() || line == "formatter:" {
            continue;
        }
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_string();
        let mut parts = rest.split(',');
        let kind = match parts.next().map(|s| s.trim()) {
            Some("string") => ParamKind::String,
            Some("bool") => ParamKind::Bool,
            Some("list") => ParamKind::List,
            Some("none") => ParamKind::None,
            _ => continue,
        };
        let mut required = false;
        let mut default = None;
        for modifier in parts {
            let m = modifier.trim();
            if m == "required" {
                required = true;
            } else if let Some(val) = m.strip_prefix("default=") {
                default = parse_default(val.trim());
            }
        }
        params.push(ParamSpec {
            name,
            kind,
            required,
            default,
        });
    }
    params
}

fn parse_default(s: &str) -> Option<Value> {
    match s {
        "true" => Some(Value::Bool(true)),
        "false" => Some(Value::Bool(false)),
        _ if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 => {
            Some(Value::Str(s[1..s.len() - 1].to_string()))
        }
        _ => None,
    }
}

/// Cached scan of `.steer/templates/default/*.j2.md`, parsed into
/// [`NodeTemplate`]s keyed by node name. Each CLI invocation is a fresh
/// process so the cache initialises once per run.
fn node_templates() -> &'static HashMap<String, NodeTemplate> {
    static CACHE: std::sync::OnceLock<HashMap<String, NodeTemplate>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| {
        let dir = std::path::Path::new(".steer/templates/default");
        let mut map = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Some(stem) = name.strip_suffix(".j2.md") {
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            map.insert(stem.to_string(), parse_template(&content));
                        }
                    }
                }
            }
        }
        map
    })
}

fn read_templates_dir(dir: &std::path::Path) -> HashMap<String, NodeTemplate> {
    let mut map = HashMap::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if let Some(stem) = name.strip_suffix(".j2.md") {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        map.insert(stem.to_string(), parse_template(&content));
                    }
                }
            }
        }
    }
    map
}

fn workflow_node_templates(dir_name: &str) -> HashMap<String, NodeTemplate> {
    read_templates_dir(&std::path::Path::new(".steer/templates").join(dir_name))
}

/// Resolve the [`NodeTemplate`] for a node, in priority order:
/// 1. a file in `.steer/templates/default/<callee>.j2.md`;
/// 2. the hardcoded fallback ([`fallback_template`]);
/// 3. a generic task-like template (minimal formatter + [`TASK_BODY`]).
pub fn resolve_template(callee: &str) -> NodeTemplate {
    resolve_template_with_meta(callee, &WorkflowMeta::default())
}

/// Resolve the [`NodeTemplate`] for a node, considering runtime workflow meta:
/// 1. a file in `.steer/templates/<meta.template_dir>/<callee>.j2.md`;
/// 2. a file in `.steer/templates/default/<callee>.j2.md`;
/// 3. the hardcoded fallback ([`fallback_template`]);
/// 4. a generic task-like template (minimal formatter + [`TASK_BODY`]).
pub fn resolve_template_with_meta(callee: &str, meta: &WorkflowMeta) -> NodeTemplate {
    if let Some(dir) = &meta.template_dir {
        if dir != "default" {
            if let Some(t) = workflow_node_templates(dir).get(callee) {
                return t.clone();
            }
        }
    }
    if let Some(t) = node_templates().get(callee) {
        return t.clone();
    }
    fallback_template(callee).unwrap_or(NodeTemplate {
        params: vec![ParamSpec {
            name: "instruction".into(),
            kind: ParamKind::String,
            required: true,
            default: None,
        }],
        body: TASK_BODY.to_string(),
    })
}

/// Hardcoded fallback templates (used when `default/` files are absent).
fn fallback_template(name: &str) -> Option<NodeTemplate> {
    let (params, body) = match name {
        "task" => (
            vec![
                spec("instruction", ParamKind::String, true, None),
                spec("return", ParamKind::String, false, None),
                spec("check", ParamKind::String, false, None),
                spec("produce", ParamKind::List, false, None),
            ],
            TASK_BODY,
        ),
        "ask" => (
            vec![
                spec("instruction", ParamKind::String, true, None),
                spec("return", ParamKind::String, false, None),
                spec("check", ParamKind::String, false, None),
            ],
            ASK_BODY,
        ),
        "command" => (
            vec![
                spec("instruction", ParamKind::String, true, None),
                spec("return", ParamKind::String, false, None),
                spec("produce", ParamKind::List, false, None),
                spec("check", ParamKind::String, false, None),
            ],
            COMMAND_BODY,
        ),
        "collect" => (
            vec![
                spec("instruction", ParamKind::String, true, None),
                spec("return", ParamKind::String, false, None),
                spec("check", ParamKind::String, false, None),
            ],
            COLLECT_BODY,
        ),
        "print" => (
            vec![
                spec("instruction", ParamKind::String, true, None),
                spec("return", ParamKind::None, false, None),
            ],
            PRINT_BODY,
        ),
        "judge" => (
            vec![
                spec("instruction", ParamKind::String, true, None),
                spec("return", ParamKind::IntrinsicBool, false, None),
            ],
            JUDGE_BODY,
        ),
        _ => return None,
    };
    Some(NodeTemplate {
        params,
        body: body.to_string(),
    })
}

fn spec(name: &str, kind: ParamKind, required: bool, default: Option<Value>) -> ParamSpec {
    ParamSpec {
        name: name.into(),
        kind,
        required,
        default,
    }
}

// ---- built-in action-node templates (const fallbacks) ----
//
// Each template renders to Markdown that the agent reads as its instruction.
// The context carries: `instruction` (positional arg), `target` (the variable
// to `steer set` or `<var>`), and runtime-rendered named args such as `return`
// and `produce` when present. `check` is handled by the VM, not Jinja.

/// `task` — the universal agent primitive. Do work, optionally report a value,
/// optionally verify, optionally produce files.
const TASK_BODY: &str = "\
{{ instruction }}
{% if return %}- Report the result via `steer set {{ target }}` in this format: {{ return }}
{% endif %}{% if produce %}- Produce these files: {% for f in produce %}{{ f }} {% endfor %}
{% endif %}";

/// `ask` — obtain a value from the human user (e.g. via AskUserQuestion).
const ASK_BODY: &str = "\
**Ask the user:** {{ instruction }}
{% if return %}- Report their answer via `steer set {{ target }}` in this format: {{ return }}
{% endif %}";

/// `command` — run a shell command and capture its output.
const COMMAND_BODY: &str = "\
**Shell command:** {{ instruction }}
{% if return %}- Report the output via `steer set {{ target }}` in this format: {{ return }}
{% endif %}{% if produce %}- Expected files: {% for f in produce %}{{ f }} {% endfor %}
{% endif %}";

/// `collect` — a reasoning value op: the agent investigates/analyzes on its own
/// and reports the value that work produces (unlike `ask`, sourced from the
/// user, or `command`, sourced from the shell).
const COLLECT_BODY: &str = "\
{{ instruction }}
- Reasoning op: derive the value from YOUR OWN investigation and analysis — read the relevant files or code, trace or reproduce the behavior, reason it through. This is not `ask` (the user) or `command` (a shell). Ground the answer in what you examined; do not guess.
{% if return %}- Report the result via `steer set {{ target }}` in this format: {{ return }}
{% endif %}";

/// `print` — output for side effects; no value, no verification.
const PRINT_BODY: &str = "{{ instruction }}";

/// `judge` — a boolean judgment. Unlike value nodes it has no `return=`
/// argument; a boolean is its intrinsic result.
const JUDGE_BODY: &str = "\
{{ instruction }}

Answer with only `true` or `false`. Set it via `steer set {{ target }}`.";

/// Render the instruction string for a single call.
///
/// `into` is the variable that receives the call's value when the call is
/// assigned or returned; it is exposed to the template as `target` so the
/// agent knows which variable to `steer set`.
///
/// The template is resolved by [`resolve_template`]: file → fallback const →
/// generic task. Unknown callees use the generic task template.
pub fn render_call(
    call: &Call,
    into: Option<&str>,
    vars: Option<&HashMap<String, Value>>,
    meta: &WorkflowMeta,
    instance: &str,
) -> String {
    let tmpl_def = resolve_template_with_meta(&call.callee, meta);
    let tmpl = Template::parse(&tmpl_def.body).expect("template body must parse");
    tmpl.render(&build_context(call, into, vars, instance))
}

/// Build the template context from a call's arguments: the first positional
/// argument is the `instruction`, named arguments map by name, and `target`
/// carries the assignment variable, or `<var>` when there is none. The `return`
/// argument is only exposed when there is a real receiver; bare calls do not
/// produce a `steer set <var>` prompt. When `vars` is provided for a real run,
/// argument expressions are evaluated against the current scope; otherwise they
/// degrade to static placeholders.
fn build_context(
    call: &Call,
    into: Option<&str>,
    vars: Option<&HashMap<String, Value>>,
    instance: &str,
) -> HashMap<String, Value> {
    let mut ctx = HashMap::new();
    ctx.insert(
        "target".to_string(),
        Value::Str(into.unwrap_or("<var>").to_string()),
    );
    ctx.insert("instance".to_string(), Value::Str(instance.to_string()));
    if let Some(CallArg::Positional(e)) = call.args.first().map(|a| &a.value) {
        ctx.insert("instruction".to_string(), arg_value(e, vars));
    }
    for a in &call.args {
        if let CallArg::Named { name, value } = &a.value {
            if name == "check" || (name == "return" && into.is_none()) {
                continue;
            }
            ctx.insert(name.clone(), arg_value(value, vars));
        }
    }
    ctx
}

/// Evaluate an argument expression, using runtime scope when available and
/// falling back to a placeholder on failure.
fn arg_value(e: &Spanned<Expr>, vars: Option<&HashMap<String, Value>>) -> Value {
    match vars {
        Some(v) => crate::value::eval(&e.value, v).unwrap_or_else(|_| eval_literal(e)),
        None => eval_literal(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn ctx(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn render(src: &str, pairs: &[(&str, Value)]) -> String {
        Template::parse(src)
            .expect("template parses")
            .render(&ctx(pairs))
    }

    #[test]
    fn plain_text_passes_through() {
        assert_eq!(render("hello world", &[]), "hello world");
    }

    #[test]
    fn var_interpolation() {
        assert_eq!(
            render("hi {{ name }}", &[("name", Value::Str("bob".into()))]),
            "hi bob"
        );
    }

    #[test]
    fn missing_var_renders_empty() {
        assert_eq!(render("hi {{ name }}", &[]), "hi ");
    }

    #[test]
    fn if_true_and_false() {
        let tmpl = "{% if flag %}ON{% else %}OFF{% endif %}";
        assert_eq!(render(tmpl, &[("flag", Value::Bool(true))]), "ON");
        assert_eq!(render(tmpl, &[("flag", Value::Bool(false))]), "OFF");
        assert_eq!(render(tmpl, &[]), "OFF"); // absent => falsy
    }

    #[test]
    fn if_without_else() {
        let tmpl = "x{% if flag %}Y{% endif %}z";
        assert_eq!(render(tmpl, &[("flag", Value::Bool(true))]), "xYz");
        assert_eq!(render(tmpl, &[("flag", Value::Bool(false))]), "xz");
    }

    #[test]
    fn for_loop_over_list() {
        let tmpl = "{% for f in files %}[{{ f }}] {% endfor %}";
        let v = render(
            tmpl,
            &[(
                "files",
                Value::List(vec![Value::Str("a".into()), Value::Str("b".into())]),
            )],
        );
        assert_eq!(v, "[a] [b] ");
    }

    #[test]
    fn for_over_missing_renders_nothing() {
        let tmpl = "{% for f in files %}{{ f }}{% endfor %}end";
        assert_eq!(render(tmpl, &[]), "end");
    }

    #[test]
    fn nested_if_in_for() {
        let tmpl = "{% for n in xs %}{% if n %}{{ n }};{% endif %}{% endfor %}";
        let v = render(
            tmpl,
            &[(
                "xs",
                Value::List(vec![
                    Value::Str("a".into()),
                    Value::Str(String::new()),
                    Value::Str("b".into()),
                ]),
            )],
        );
        assert_eq!(v, "a;b;");
    }

    #[test]
    fn err_unterminated_expr() {
        assert_eq!(
            Template::parse("hi {{ name").unwrap_err(),
            TemplateError::UnterminatedExpr
        );
    }

    #[test]
    fn err_unterminated_tag() {
        // a statement tag that never closes with `%}`
        assert_eq!(
            Template::parse("{% if x ").unwrap_err(),
            TemplateError::UnterminatedTag
        );
    }

    #[test]
    fn err_unclosed_if_block() {
        // `{% if x %}` with no matching `{% endif %}`
        assert!(matches!(
            Template::parse("{% if x %}body"),
            Err(TemplateError::WrongClose {
                expected: "endif",
                ..
            })
        ));
    }

    #[test]
    fn err_wrong_close() {
        // for closed by endif
        assert!(matches!(
            Template::parse("{% for x in xs %}{% endif %}"),
            Err(TemplateError::WrongClose {
                expected: "endfor",
                ..
            })
        ));
    }

    #[test]
    fn err_unmatched_close() {
        assert!(matches!(
            Template::parse("hello {% endif %}"),
            Err(TemplateError::UnmatchedClose(_))
        ));
    }

    // ---- render_call ----

    fn first_call_instruction(src: &str) -> String {
        let m = steer_syntax::parse(src).unwrap();
        match &m.body[0].value {
            steer_syntax::ast::Stmt::Call(c) => {
                render_call(c, None, None, &WorkflowMeta::default(), "<name>")
            }
            _ => panic!("not a call statement"),
        }
    }

    #[test]
    fn render_bare_task() {
        let out = first_call_instruction("task(\"do something\")\n");
        assert!(out.contains("do something"));
        // no return/check/produce => no extra bullet lines
        assert!(!out.contains("Set the result"));
        assert!(!out.contains("Verify"));
    }

    #[test]
    fn render_task_with_all_params() {
        let out = first_call_instruction(
            "task(\"do it\", return=\"path\", produce=[\"a\", \"b\"], check=\"ok\")\n",
        );
        assert!(out.contains("do it"));
        assert!(!out.contains("steer set <var>"));
        assert!(out.contains("Produce these files"));
        assert!(out.contains("a"));
        assert!(out.contains("b"));
        assert!(!out.contains("Verify: ok"));
        assert!(!out.contains("steer set checked"));
        assert!(!out.contains("sub-agent"));
    }

    #[test]
    fn render_print_uses_minimal_body() {
        let out = first_call_instruction("print(\"hello\")\n");
        assert_eq!(out, "hello");
    }

    #[test]
    fn render_unknown_callee_falls_back_to_value_template() {
        let out = first_call_instruction("custom(\"thing\", return=\"x\")\n");
        assert!(out.contains("thing"));
        assert!(!out.contains("steer set <var>"));
    }

    #[test]
    fn assigned_value_call_renders_set_prompt_with_target() {
        let m = steer_syntax::parse("out = task(\"do\", return=\"path\")\n").unwrap();
        let steer_syntax::ast::Stmt::Assign { target, value } = &m.body[0].value else {
            panic!("not an assignment")
        };
        let steer_syntax::ast::Expr::Call(c) = &value.value else {
            panic!("not a call")
        };
        let out = render_call(
            c,
            Some(target.as_str()),
            None,
            &WorkflowMeta::default(),
            "<name>",
        );
        assert!(out.contains("steer set out"));
        assert!(out.contains("path"));
    }

    #[test]
    fn render_judge_asks_for_boolean_and_targets_var() {
        let m = steer_syntax::parse("passed = judge(\"is the build green?\")\n").unwrap();
        let stmt = &m.body[0].value;
        let steer_syntax::ast::Stmt::Assign { target, value } = stmt else {
            panic!("not an assignment")
        };
        let steer_syntax::ast::Expr::Call(c) = &value.value else {
            panic!("not a call")
        };
        let out = render_call(
            c,
            Some(target.as_str()),
            None,
            &WorkflowMeta::default(),
            "<name>",
        );
        assert!(out.contains("is the build green?"));
        assert!(out.contains("`true` or `false`"));
        assert!(out.contains("steer set passed"));
        // judge carries no verify/return lines
        assert!(!out.contains("Verify"));
        assert!(!out.contains("Expected files"));
    }

    #[test]
    fn builtin_templates_all_parse() {
        // A typo in a built-in const template would panic at the first
        // render_call; this test catches it at `cargo test` time instead.
        for body in [
            TASK_BODY,
            ASK_BODY,
            COMMAND_BODY,
            COLLECT_BODY,
            PRINT_BODY,
            JUDGE_BODY,
        ] {
            Template::parse(body).expect("built-in template must parse");
        }
    }

    #[test]
    fn value_nodes_differentiate_by_source() {
        // Each sugar node carries a distinct "source of value" hint.
        let task_out = first_call_instruction("task(\"do\")\n");
        let ask_out = first_call_instruction("ask(\"do\", return=\"x\")\n");
        let cmd_out = first_call_instruction("command(\"do\", return=\"x\")\n");
        let col_out = first_call_instruction("collect(\"do\", return=\"x\")\n");
        assert!(!task_out.contains("Ask the user"));
        assert!(ask_out.contains("Ask the user"));
        assert!(cmd_out.contains("Shell command"));
        assert!(col_out.contains("Reasoning op"));
        // collect must not read like ask or command.
        assert!(!col_out.contains("Ask the user"));
        assert!(!col_out.contains("Shell command"));
    }

    #[test]
    fn collect_template_conveys_reasoning_semantic() {
        // `collect` is a value op whose value must come from the agent's OWN
        // reasoning/investigation — its defining distinction from `ask` (user)
        // and `command` (shell). The rendered prompt must say so, not merely
        // echo the instruction plus a report-back line.
        let out = first_call_instruction("collect(\"find the root cause\", return=\"summary\")\n");
        assert!(
            out.contains("find the root cause"),
            "instruction missing: {out}"
        );
        assert!(
            out.contains("Reasoning op") || out.contains("investigation"),
            "collect reasoning semantic missing: {out}",
        );
        assert!(
            !out.contains("Ask the user"),
            "collect looks like ask: {out}"
        );
        assert!(
            !out.contains("Shell command"),
            "collect looks like command: {out}"
        );
    }

    #[test]
    fn collect_file_template_conveys_reasoning_semantic() {
        // The shipped file template — used at runtime when
        // `.steer/templates/default/` is present — must carry the same
        // reasoning semantic as the fallback const, not just a bare
        // instruction + report.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(".steer/templates/default/collect.j2.md");
        let body = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("collect template missing at {}", path.display()));
        let lower = body.to_lowercase();
        assert!(
            lower.contains("reasoning op") || lower.contains("investigation"),
            "collect file template lacks reasoning semantic:\n{body}",
        );
    }

    #[test]
    fn interpolation_in_instruction_preserved_as_placeholder() {
        let out = first_call_instruction("task(\"fix {f}\")\n");
        assert!(out.contains("fix {f}"));
    }

    #[test]
    fn workflow_template_dir_overrides_default() {
        let dir_name = format!("test-meta-{}", std::process::id());
        let dir = std::path::Path::new(".steer/templates").join(&dir_name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("task.j2.md"), "CUSTOM {{ instruction }}").unwrap();

        let m = steer_syntax::parse("task(\"body\")\n").unwrap();
        let steer_syntax::ast::Stmt::Call(c) = &m.body[0].value else {
            panic!("not a call statement")
        };
        let meta = WorkflowMeta {
            template_dir: Some(dir_name),
        };
        let out = render_call(c, None, None, &meta, "<name>");
        assert_eq!(out, "CUSTOM body");

        let _ = std::fs::remove_dir_all(dir);
    }
}
