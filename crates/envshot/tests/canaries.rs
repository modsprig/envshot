use envshot_core::{PrivacyPolicy, apply_privacy};
use envshot_model::{
    CaptureProfile, CollectorIssue, IssueCode, Section, SectionData, SectionStatus,
};

fn hostile_snapshot() -> envshot_model::Snapshot {
    let mut snapshot =
        envshot::capture_with_privacy(CaptureProfile::Minimal, &PrivacyPolicy::off());
    let issues = vec![
        CollectorIssue::new(IssueCode::new("parse_failed").expect("static")).with_detail(
            "user S3CR3T_CANARY_USERNAME at home S3CR3T_CANARY_HOMEPATH token S3CR3T_CANARY_TOKEN",
        ),
    ];
    snapshot.sections.insert(
        "hostile".to_owned(),
        Section::<SectionData>::new(SectionStatus::Failed, None, issues)
            .expect("failed with issue is valid"),
    );
    snapshot
}

#[test]
fn standard_and_strict_output_contain_no_canaries() {
    for policy in [PrivacyPolicy::standard(), PrivacyPolicy::strict()] {
        let json = serde_json::to_string(&apply_privacy(&hostile_snapshot(), &policy))
            .expect("must serialize");
        envshot_core::testing::assert_no_canaries(&json);
    }
}

#[test]
fn off_output_does_contain_the_canaries() {
    let json = serde_json::to_string(&apply_privacy(&hostile_snapshot(), &PrivacyPolicy::off()))
        .expect("must serialize");
    assert!(
        json.contains("S3CR3T_CANARY_TOKEN"),
        "off keeps captured detail"
    );
}

#[test]
fn no_pseudonymous_host_identifier_exists() {
    let json =
        serde_json::to_string(&envshot::capture(CaptureProfile::Minimal)).expect("must serialize");
    for marker in [
        "machine_id",
        "machine-id",
        "host_id",
        "hostId",
        "serial_number",
        "system_uuid",
        "device_uuid",
        "hardware_id",
    ] {
        assert!(
            !json.to_lowercase().contains(marker),
            "pseudonymous id marker {marker} found"
        );
    }
}
