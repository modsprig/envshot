use std::process::Command;

fn run(args: &[&str]) -> (i32, String, String) {
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
fn capture_defaults_exit_zero_with_json_snapshot() {
    let (code, stdout, stderr) = run(&["capture"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be JSON");
    assert_eq!(parsed["capture"]["privacy"], "standard");
    assert_eq!(parsed["capture"]["profile"], "standard");
    assert!(parsed["sections"]["os"].is_object(), "MVP sections present");

    assert!(stderr.is_empty());
}

#[test]
fn privacy_off_warns_on_stderr_and_records_off() {
    let (code, stdout, stderr) = run(&["capture", "--privacy", "off"]);
    assert_eq!(code, 0);
    assert!(stderr.contains("warning"), "stderr must warn: {stderr}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be JSON");
    assert_eq!(parsed["capture"]["privacy"], "off");
}

#[test]
fn invalid_profile_exits_two() {
    let (code, stdout, stderr) = run(&["capture", "--profile", "bogus"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("unknown --profile"));
}

#[test]
fn invalid_privacy_exits_two() {
    let (code, _, _) = run(&["capture", "--privacy", "bogus"]);
    assert_eq!(code, 2);
}

#[test]
fn output_file_writes_snapshot_and_keeps_stdout_empty() {
    let file = std::env::temp_dir().join("envshot-cli-capture-test.json");
    let (code, stdout, stderr) = run(&["capture", "-o", file.to_str().expect("utf-8 path")]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.is_empty(), "stdout stays empty with -o");
    let content = std::fs::read_to_string(&file).expect("output file readable");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("file must be JSON");
    assert_eq!(parsed["schema"]["name"], "envshot.snapshot");
    let _ = std::fs::remove_file(&file);
}

#[test]
fn empty_test_profile_yields_envelope() {
    let (code, stdout, _) = run(&["capture", "--profile", "empty-test"]);
    assert_eq!(code, 0);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be JSON");
    assert_eq!(parsed["sections"], serde_json::json!({}));
}

#[test]
fn minimal_profile_runs() {
    let (code, stdout, _) = run(&["capture", "--profile", "minimal"]);
    assert_eq!(code, 0);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be JSON");
    assert_eq!(parsed["capture"]["profile"], "minimal");
}

#[test]
fn strict_privacy_runs() {
    let (code, stdout, _) = run(&["capture", "--privacy", "strict"]);
    assert_eq!(code, 0);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be JSON");
    assert_eq!(parsed["capture"]["privacy"], "strict");
}

#[test]
fn pretty_json_format_runs() {
    let (code, stdout, _) = run(&["capture", "--format", "pretty-json"]);
    assert_eq!(code, 0);
    assert!(stdout.contains('\n'), "pretty output must span lines");
    serde_json::from_str::<serde_json::Value>(&stdout).expect("stdout must still be JSON");
}
