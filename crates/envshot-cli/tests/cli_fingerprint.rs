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
fn text_output_is_the_envelope_value() {
    let args = vec![
        "fingerprint".to_owned(),
        fixture_path("complete-basic.json"),
    ];
    let (code, stdout, stderr) = run(&args);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.trim().starts_with("repro-v1:sha256:"),
        "unexpected output: {stdout}"
    );
}

#[test]
fn json_output_includes_the_projection() {
    let args = vec![
        "fingerprint".to_owned(),
        "--format".to_owned(),
        "json".to_owned(),
        fixture_path("complete-basic.json"),
    ];
    let (code, stdout, _) = run(&args);
    assert_eq!(code, 0);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be JSON");
    assert_eq!(parsed["fingerprint_schema"]["name"], "envshot.fingerprint");
    assert_eq!(parsed["fingerprint_schema"]["version"], 1);
    assert!(
        parsed["fingerprint"]
            .as_str()
            .expect("string")
            .starts_with("repro-v1:sha256:")
    );
    assert_eq!(parsed["projection"]["facts"]["os.family"], "windows");
}

#[test]
fn malformed_input_exits_three() {
    let args = vec!["fingerprint".to_owned(), "does-not-exist.json".to_owned()];
    let (code, _, stderr) = run(&args);
    assert_eq!(code, 3);
    assert!(stderr.contains("error:"));
}

#[test]
fn unknown_profile_exits_two() {
    let args = vec![
        "fingerprint".to_owned(),
        "--profile".to_owned(),
        "repro-v2".to_owned(),
        fixture_path("complete-basic.json"),
    ];
    let (code, _, stderr) = run(&args);
    assert_eq!(code, 2);
    assert!(stderr.contains("unknown --profile"));
}
