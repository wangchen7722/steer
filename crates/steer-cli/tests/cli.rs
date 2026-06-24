//! Integration tests for the `steer` CLI binary.

use std::process::Command;

/// Path to the built `steer` binary, provided by cargo at test time.
fn steer() -> Command {
    Command::new(env!("CARGO_BIN_EXE_steer"))
}

/// Write `contents` to a uniquely-named temp file and return its path.
fn write_tmp(name: &str, contents: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("steer-it-{name}-{}.steer", std::process::id()));
    std::fs::write(&path, contents).expect("write temp file");
    path
}

#[test]
fn validate_valid_file_exits_zero_and_reports_ok() {
    let path = write_tmp("valid", "task(\"do something\", return=\"path\")\n");
    let out = steer()
        .args(["workflow", "validate"])
        .arg(&path)
        .output()
        .expect("run steer");
    assert!(out.status.success(), "expected success");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("OK"), "stdout was: {stdout}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn validate_value_task_without_return_exits_nonzero_with_message() {
    let path = write_tmp("no-return", "x = task(\"do something\")\n");
    let out = steer()
        .args(["workflow", "validate"])
        .arg(&path)
        .output()
        .expect("run steer");
    assert!(!out.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("return="), "stderr was: {stderr}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn validate_parse_error_exits_nonzero_with_location() {
    let path = write_tmp("parse-err", "x =\n");
    let out = steer()
        .args(["workflow", "validate"])
        .arg(&path)
        .output()
        .expect("run steer");
    assert!(!out.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("at line"), "stderr was: {stderr}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn validate_missing_file_exits_nonzero() {
    let out = steer()
        .args(["workflow", "validate"])
        .arg("/nonexistent/steer/nope.steer")
        .output()
        .expect("run steer");
    assert!(!out.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("cannot read"), "stderr was: {stderr}");
}

#[test]
fn simulate_renders_instructions_in_order() {
    let path = write_tmp(
        "sim",
        "task(\"first\")\nprint(\"second\")\nask(\"third\", return=\"x\")\n",
    );
    let out = steer()
        .args(["workflow", "simulate"])
        .arg(&path)
        .output()
        .expect("run steer");
    assert!(out.status.success(), "expected success");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("first"), "stdout was: {stdout}");
    assert!(stdout.contains("second"), "stdout was: {stdout}");
    assert!(stdout.contains("third"), "stdout was: {stdout}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn simulate_parse_error_exits_nonzero() {
    let path = write_tmp("sim-err", "x =\n");
    let out = steer()
        .args(["workflow", "simulate"])
        .arg(&path)
        .output()
        .expect("run steer");
    assert!(!out.status.success(), "expected failure");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn instance_runs_end_to_end() {
    // Drive a small workflow to completion via separate CLI invocations,
    // simulating the agent with `set` + `check`.
    let tmp = std::env::temp_dir().join(format!("steer-cli-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("make tmp dir");
    let wf = tmp.join("wf.steer");
    std::fs::write(&wf, "x = ask(\"what?\", return=\"str\")\nprint(x)\n").unwrap();

    let run = |args: &[&str]| {
        steer()
            .args(args)
            .current_dir(&tmp)
            .output()
            .expect("run steer")
    };

    // start
    let out = run(&["instance", "start", "wf.steer", "it"]);
    assert!(
        out.status.success(),
        "start failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // step -> pauses at ask
    let out = run(&["instance", "step", "it"]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("what?"));

    // set the value (the agent's response)
    let out = run(&["instance", "set", "it", "x", "answer"]);
    assert!(out.status.success());

    // check -> advances past the value op
    let out = run(&["instance", "check", "it"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("advanced"));

    // step -> pauses at print, with x resolved
    let out = run(&["instance", "step", "it"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("answer"));

    // check the print (auto-advances), then step -> complete
    run(&["instance", "check", "it"]);
    let out = run(&["instance", "step", "it"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("complete"));

    // status reflects completion
    let out = run(&["instance", "status", "it"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("complete"));

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn instance_for_loop_check_requires_per_iteration_report() {
    // Mirror .steer/workflows/demo.steer's two-file for loop: each iteration
    // re-enters the same check-bearing task. Passing file1 must NOT let file2's
    // check advance without a fresh report.
    let tmp = std::env::temp_dir().join(format!("steer-cli-loop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("make tmp dir");
    let wf = tmp.join("wf.steer");
    std::fs::write(
        &wf,
        "for f in [\"file1\", \"file2\"]\n  task(\"fix {f}\", check=\"confirm {f} is fixed\")\nend\n",
    )
    .unwrap();

    let run = |args: &[&str]| {
        steer()
            .args(args)
            .current_dir(&tmp)
            .output()
            .expect("run steer")
    };

    let out = run(&["instance", "start", "wf.steer", "it"]);
    assert!(
        out.status.success(),
        "start failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // step -> "fix file1"
    let out = run(&["instance", "step", "it"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("fix file1"));

    // check -> verification instruction (not yet reported)
    let out = run(&["instance", "check", "it"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("confirm file1 is fixed"));

    // agent reports pass for file1
    let out = run(&["instance", "set", "it", "checked", "{\"passed\":true}"]);
    assert!(out.status.success());
    let out = run(&["instance", "check", "it"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("advanced"));

    // step -> "fix file2"
    let out = run(&["instance", "step", "it"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("fix file2"));

    // Bug: file2's check would read file1's stale pass and advance. Expected:
    // the verification instruction, demanding a fresh report for file2.
    let out = run(&["instance", "check", "it"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains("confirm file2 is fixed") && !stdout.contains("advanced"),
        "file2 must require a fresh report, got: {stdout}"
    );

    // now report file2 and complete
    run(&["instance", "set", "it", "checked", "{\"passed\":true}"]);
    let out = run(&["instance", "check", "it"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("advanced"));
    let out = run(&["instance", "step", "it"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("complete"));

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn instance_start_rejects_invalid_workflow() {
    let tmp = std::env::temp_dir().join(format!("steer-cli-it2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let wf = tmp.join("bad.steer");
    std::fs::write(&wf, "x =\n").unwrap();
    let out = steer()
        .args(["instance", "start", "bad.steer", "it"])
        .current_dir(&tmp)
        .output()
        .expect("run steer");
    assert!(!out.status.success());
    let _ = std::fs::remove_dir_all(&tmp);
}

// ---- workflow file discovery under `.steer/workflows/` ----
//
// A workflow path argument is resolved as: the path as given first, then a flat
// lookup under `.steer/workflows/` by file name (auto-appending `.steer` when
// the name has no extension). Applies to `instance start`, `workflow validate`,
// and `workflow simulate`.

/// Build a temp working dir with `.steer/workflows/<name>.steer` holding
/// `content`, returning the dir path so tests can `current_dir` into it.
fn steer_workflows_dir(name: &str, content: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(format!("steer-disc-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let wf = tmp.join(".steer").join("workflows");
    std::fs::create_dir_all(&wf).expect("make workflows dir");
    std::fs::write(wf.join(format!("{name}.steer")), content).expect("write workflow");
    tmp
}

#[test]
fn instance_start_discovers_workflow_under_steer_workflows() {
    let tmp = steer_workflows_dir("disc-start", "print(\"hi\")\n");
    let out = steer()
        .args(["instance", "start", "disc-start.steer", "it"])
        .current_dir(&tmp)
        .output()
        .expect("run steer");
    assert!(
        out.status.success(),
        "start failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("started"), "stdout was: {stdout}");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn validate_discovers_workflow_by_bare_name_without_extension() {
    let tmp = steer_workflows_dir("disc-bare", "print(\"hi\")\n");
    let out = steer()
        .args(["workflow", "validate", "disc-bare"])
        .current_dir(&tmp)
        .output()
        .expect("run steer");
    assert!(
        out.status.success(),
        "validate failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("OK"), "stdout was: {stdout}");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn validate_reports_cannot_read_when_workflow_is_nowhere_to_be_found() {
    let tmp = std::env::temp_dir().join(format!("steer-none-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("make tmp dir");
    let out = steer()
        .args(["workflow", "validate", "nope.steer"])
        .current_dir(&tmp)
        .output()
        .expect("run steer");
    assert!(!out.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("cannot read"), "stderr was: {stderr}");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn explicit_cwd_file_takes_precedence_over_steer_workflows() {
    // A valid `foo.steer` sits in the CWD while `.steer/workflows/foo.steer`
    // holds a syntax error. Explicit-first resolution must read the CWD file
    // (validate succeeds), never the broken one under `.steer/workflows/`.
    let tmp = std::env::temp_dir().join(format!("steer-prec-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join(".steer").join("workflows")).expect("make workflows dir");
    std::fs::write(tmp.join("foo.steer"), "print(\"explicit\")\n").expect("write cwd file");
    std::fs::write(
        tmp.join(".steer").join("workflows").join("foo.steer"),
        "x =\n",
    )
    .expect("write workflows file");
    let out = steer()
        .args(["workflow", "validate", "foo.steer"])
        .current_dir(&tmp)
        .output()
        .expect("run steer");
    assert!(
        out.status.success(),
        "expected the explicit CWD file to win; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn instance_start_appends_context_directive() {
    let tmp = std::env::temp_dir().join(format!("steer-ctx-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("make tmp dir");
    let wf = tmp.join("ctx.steer");
    std::fs::write(
        &wf,
        "@context = \"This workflow automates root-cause analysis.\"\ntask(\"do work\")\n",
    )
    .unwrap();

    let out = steer()
        .args(["instance", "start", "ctx.steer", "it"])
        .current_dir(&tmp)
        .output()
        .expect("run steer");
    assert!(
        out.status.success(),
        "start failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("started"), "stdout was: {stdout}");
    assert!(
        stdout.contains("This workflow automates root-cause analysis."),
        "stdout was: {stdout}"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn instance_start_without_context_omits_description() {
    let tmp = std::env::temp_dir().join(format!("steer-noctx-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("make tmp dir");
    let wf = tmp.join("noctx.steer");
    std::fs::write(&wf, "task(\"do work\")\n").unwrap();

    let out = steer()
        .args(["instance", "start", "noctx.steer", "it"])
        .current_dir(&tmp)
        .output()
        .expect("run steer");
    assert!(
        out.status.success(),
        "start failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Only the "started" line, no extra paragraph
    assert!(stdout.contains("started"), "stdout was: {stdout}");
    assert!(
        !stdout.contains("do work"),
        "instruction must not leak into start output: {stdout}"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}
