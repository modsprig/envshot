use envshot_model::{
    CaptureProfile, PrivacyProfile, SchemaIdentifier, SnapshotReadError, read_snapshot,
};

const ENVELOPE_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/snapshots/v0.1/empty-test-envelope.json"
));

#[test]
fn fixture_identifies_schema_producer_and_capture_metadata() {
    let snapshot = read_snapshot(ENVELOPE_FIXTURE).expect("fixture must read");

    assert_eq!(snapshot.schema, SchemaIdentifier::current());
    assert_eq!(snapshot.schema.name, "envshot.snapshot");
    assert_eq!(snapshot.schema.major, 0);
    assert_eq!(snapshot.schema.minor, 1);

    assert_eq!(snapshot.producer.name, "envshot");
    assert_eq!(snapshot.producer.version, "1.0.0");

    assert_eq!(snapshot.capture.profile, CaptureProfile::EmptyTest);
    assert_eq!(snapshot.capture.privacy, PrivacyProfile::Standard);
}

#[test]
fn reader_accepts_supported_major_and_preserves_schema_identity() {
    let snapshot = read_snapshot(ENVELOPE_FIXTURE).expect("fixture must read");

    assert_eq!(snapshot.schema, SchemaIdentifier::current());
}

#[test]
fn reader_rejects_unsupported_future_major_with_typed_error() {
    let document = ENVELOPE_FIXTURE.replace(r#""major":0"#, r#""major":1"#);
    assert_ne!(document, ENVELOPE_FIXTURE, "mutation must apply");

    let error = read_snapshot(&document).expect_err("future major must be rejected");

    assert_eq!(error.code(), "snapshot.read.unsupported_schema_major");
    match error {
        SnapshotReadError::UnsupportedSchemaMajor { found, supported } => {
            assert_eq!(found, 1);
            assert_eq!(supported, 0);
        }
        other @ SnapshotReadError::Malformed { .. } => {
            panic!("wrong error variant: {other:?}")
        }
    }
}

#[test]
fn reader_rejects_malformed_input_with_typed_error() {
    let error = read_snapshot("{ not json").expect_err("malformed input must be rejected");

    assert_eq!(error.code(), "snapshot.read.malformed");
}

#[test]
fn reader_rejects_document_without_schema_identifier() {
    let error = read_snapshot(r#"{"producer":{"name":"envshot","version":"0.0.0"}}"#)
        .expect_err("missing schema must be rejected");

    assert_eq!(error.code(), "snapshot.read.malformed");
}

#[test]
fn reader_rejects_non_numeric_schema_major() {
    let document = ENVELOPE_FIXTURE.replace(r#""major":0"#, r#""major":"zero""#);
    assert_ne!(document, ENVELOPE_FIXTURE, "mutation must apply");

    let error = read_snapshot(&document).expect_err("non-numeric major must be rejected");

    assert_eq!(error.code(), "snapshot.read.malformed");
}

#[test]
fn missing_privacy_metadata_defaults_to_standard() {
    let document = ENVELOPE_FIXTURE.replace(r#","privacy":"standard""#, "");
    assert_ne!(document, ENVELOPE_FIXTURE, "mutation must apply");

    let snapshot = read_snapshot(&document).expect("privacy absence must be tolerated");

    assert_eq!(snapshot.capture.privacy, PrivacyProfile::Standard);
}

#[test]
fn reader_rejects_invalid_document_body_after_schema_check() {
    let document = ENVELOPE_FIXTURE.replace(r#""name":"envshot""#, r#""name":42"#);
    assert_ne!(document, ENVELOPE_FIXTURE, "mutation must apply");

    let error = read_snapshot(&document).expect_err("invalid body must be rejected");

    assert_eq!(error.code(), "snapshot.read.malformed");
}
