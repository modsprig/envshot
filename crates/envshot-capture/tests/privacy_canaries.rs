use envshot_capture::capture;
use envshot_model::CaptureProfile;

fn canary_values() -> Vec<String> {
    const CANARY_KEYS: [&str; 10] = [
        "USERNAME",
        "COMPUTERNAME",
        "USERDOMAIN",
        "USER",
        "LOGNAME",
        "HOME",
        "USERPROFILE",
        "USER DNSDOMAIN",
        "NETBIOS",
        "SESSIONNAME",
    ];
    CANARY_KEYS
        .iter()
        .filter_map(|key| std::env::var(key).ok())
        .filter(|value| value.len() > 3)
        .collect()
}

#[test]
fn standard_snapshot_contains_no_host_identifiers() {
    let canaries = canary_values();
    assert!(
        !canaries.is_empty(),
        "this host must expose at least one identity canary for the test to be meaningful"
    );

    let snapshot = capture(CaptureProfile::Standard);
    let json = serde_json::to_string(&snapshot).expect("must serialize");

    for canary in &canaries {
        assert!(
            !json.contains(canary.as_str()),
            "standard-privacy output leaked the identity canary {canary:?}: {json}"
        );
    }
}
