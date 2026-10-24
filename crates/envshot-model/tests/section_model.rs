use envshot_model::{
    CollectorIssue, IssueCode, Section, SectionError, SectionStatus, read_snapshot,
};

fn issue(code: &str) -> CollectorIssue {
    let code = IssueCode::new(code).expect("test codes are non-empty");
    CollectorIssue::new(code)
}

#[test]
fn available_section_carries_data_and_round_trips() {
    let section = Section::available(42_u32);

    assert_eq!(section.status(), SectionStatus::Available);
    assert_eq!(section.data(), Some(&42));
    assert!(section.issues().is_empty());

    let json = serde_json::to_string(&section).expect("serialization must not fail");

    assert_eq!(
        json,
        r#"{"status":"available","data":42,"provenance":[],"issues":[]}"#
    );

    let parsed: Section<u32> = serde_json::from_str(&json).expect("own serialization must parse");
    assert_eq!(parsed, section);
}

#[test]
fn partial_section_carries_data_and_issue() {
    let section = Section::partial("cache".to_owned(), vec![issue("parse_failed")])
        .expect("data + issue is valid");

    assert_eq!(section.status(), SectionStatus::Partial);
    assert_eq!(section.data(), Some(&"cache".to_owned()));
    assert_eq!(section.issues().len(), 1);
    assert_eq!(section.issues()[0].code.as_str(), "parse_failed");
}

#[test]
fn non_collecting_states_carry_no_data_and_round_trip() {
    let cases = [
        (
            SectionStatus::Unsupported,
            Section::<String>::unsupported(vec![issue("not_implemented")]),
        ),
        (
            SectionStatus::PermissionDenied,
            Section::<String>::permission_denied(vec![issue("access_denied")]),
        ),
        (
            SectionStatus::Unavailable,
            Section::<String>::unavailable(vec![]),
        ),
        (
            SectionStatus::Failed,
            Section::<String>::failed(vec![issue("collector_failed")]),
        ),
    ];

    for (status, section) in cases {
        assert_eq!(section.status(), status);
        assert_eq!(section.data(), None, "{status:?} must carry no data");

        let json = serde_json::to_string(&section).expect("serialization must not fail");
        let parsed: Section<String> =
            serde_json::from_str(&json).expect("own serialization must parse");
        assert_eq!(parsed, section, "{status:?} must round-trip");
    }
}

#[test]
fn available_without_data_is_rejected() {
    let error = Section::<()>::new(SectionStatus::Available, None, vec![])
        .expect_err("available requires data");

    assert!(matches!(error, SectionError::DataRequired { .. }));
}

#[test]
fn partial_without_data_or_issues_is_rejected() {
    let no_data = Section::<u32>::new(SectionStatus::Partial, None, vec![issue("x")])
        .expect_err("partial requires data");
    assert!(matches!(no_data, SectionError::DataRequired { .. }));

    let no_issues = Section::partial(7_u32, vec![]).expect_err("partial requires an issue");
    assert!(matches!(no_issues, SectionError::IssueRequired));

    let neither = Section::<u32>::new(SectionStatus::Partial, None, vec![])
        .expect_err("partial requires both");
    assert!(matches!(neither, SectionError::DataRequired { .. }));
}

#[test]
fn reader_rejects_available_section_without_data() {
    let document = r#"{
        "schema": {"name": "envshot.snapshot", "major": 0, "minor": 1},
        "producer": {"name": "envshot", "version": "0.0.0"},
        "capture": {"profile": "empty_test", "privacy": "standard"},
        "sections": {"os": {"status": "available"}}
    }"#;

    let error = read_snapshot(document).expect_err("invalid section must be rejected");
    assert_eq!(error.code(), "snapshot.read.malformed");
}

#[test]
fn reader_rejects_unknown_status_code() {
    let document = r#"{
        "schema": {"name": "envshot.snapshot", "major": 0, "minor": 1},
        "producer": {"name": "envshot", "version": "0.0.0"},
        "capture": {"profile": "empty_test", "privacy": "standard"},
        "sections": {"os": {"status": "fine"}}
    }"#;

    let error = read_snapshot(document).expect_err("unknown status must be rejected");
    assert_eq!(error.code(), "snapshot.read.malformed");
}

#[test]
fn reader_preserves_valid_sections_in_documents() {
    let document = r#"{
        "schema": {"name": "envshot.snapshot", "major": 0, "minor": 1},
        "producer": {"name": "envshot", "version": "0.0.0"},
        "capture": {"profile": "empty_test", "privacy": "standard"},
        "sections": {
            "memory": {"status": "unavailable", "issues": [{"code": "no_driver"}]},
            "cpu": {"status": "unsupported", "issues": []}
        }
    }"#;

    let snapshot = read_snapshot(document).expect("valid sections must read");

    let memory = snapshot.sections.get("memory").expect("memory section");
    assert_eq!(memory.status(), SectionStatus::Unavailable);
    assert_eq!(memory.issues()[0].code.as_str(), "no_driver");

    let cpu = snapshot.sections.get("cpu").expect("cpu section");
    assert_eq!(cpu.status(), SectionStatus::Unsupported);
    assert!(cpu.issues().is_empty());
}

#[test]
fn empty_issue_code_is_rejected() {
    assert!(IssueCode::new("").is_err());
}

#[test]
fn equal_sections_are_equal() {
    let a = Section::available("x".to_owned());
    let b = Section::new(SectionStatus::Available, Some("x".to_owned()), vec![]).expect("valid");
    assert_eq!(a, b);
}

#[test]
fn reader_rejects_non_array_issues() {
    let document = r#"{
        "schema": {"name": "envshot.snapshot", "major": 0, "minor": 1},
        "producer": {"name": "envshot", "version": "0.0.0"},
        "capture": {"profile": "empty_test", "privacy": "standard"},
        "sections": {"os": {"status": "failed", "issues": "broken"}}
    }"#;

    let error = read_snapshot(document).expect_err("non-array issues must be rejected");
    assert_eq!(error.code(), "snapshot.read.malformed");
}

#[test]
fn reader_rejects_non_string_issue_code() {
    let document = r#"{
        "schema": {"name": "envshot.snapshot", "major": 0, "minor": 1},
        "producer": {"name": "envshot", "version": "0.0.0"},
        "capture": {"profile": "empty_test", "privacy": "standard"},
        "sections": {"os": {"status": "failed", "issues": [{"code": 42}]}}
    }"#;

    let error = read_snapshot(document).expect_err("non-string code must be rejected");
    assert_eq!(error.code(), "snapshot.read.malformed");
}
