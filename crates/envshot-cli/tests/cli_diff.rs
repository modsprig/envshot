use std::process::Command;

fn fixture_path(name: &str) -> String {
    format!(
        "{}/../../fixtures/snapshots/v0.1/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn run(args: &[String]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_envshot"))
        .args(args)
        .output()
        .expect("binary must run");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn diff_on_stored_fixtures_exits_zero_with_text() {
    let args = vec![
        "diff".to_owned(),
        fixture_path("partial-snapshot.json"),
        fixture_path("complete-basic.json"),
    ];
    let (code, stdout, stderr) = run(&args);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("cpu:"), "groups by section: {stdout}");
    assert!(stdout.contains("memory:"), "groups by section: {stdout}");
    assert!(stdout.contains("->"), "changes render before -> after");
}

#[test]
fn identical_fixtures_report_no_changes() {
    let path = fixture_path("complete-basic.json");
    let (code, stdout, _) = run(&["diff".to_owned(), path.clone(), path.clone()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("no changes"));
}

#[test]
fn json_format_produces_the_versioned_envelope() {
    let args = vec![
        "diff".to_owned(),
        "--format".to_owned(),
        "json".to_owned(),
        fixture_path("partial-snapshot.json"),
        fixture_path("complete-basic.json"),
    ];
    let (code, stdout, _) = run(&args);
    assert_eq!(code, 0);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be JSON");
    assert_eq!(parsed["diff_schema"]["name"], "envshot.diff");
    assert_eq!(parsed["diff_schema"]["version"], 1);
    assert!(parsed["changes"].is_array());
}

#[test]
fn malformed_input_exits_three() {
    let args = vec![
        "diff".to_owned(),
        fixture_path("complete-basic.json"),
        "does-not-exist.json".to_owned(),
    ];
    let (code, _, stderr) = run(&args);
    assert_eq!(code, 3);
    assert!(stderr.contains("error:"));
}

#[test]
fn diagnostic_policy_runs() {
    let args = vec![
        "diff".to_owned(),
        "--policy".to_owned(),
        "diagnostic".to_owned(),
        fixture_path("partial-snapshot.json"),
        fixture_path("complete-basic.json"),
    ];
    let (code, stdout, _) = run(&args);
    assert_eq!(code, 0);
    assert!(stdout.contains("cpu:"), "grouped output: {stdout}");
}
