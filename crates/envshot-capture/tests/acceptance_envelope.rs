use envshot_capture::capture;
use envshot_model::{CaptureProfile, SchemaIdentifier};

fn serialize(snapshot: &envshot_model::Snapshot) -> String {
    serde_json::to_string(snapshot).expect("snapshot serialization must not fail")
}

#[test]
fn capture_with_empty_test_profile_yields_schema_identifier_and_producer_metadata() {
    let snapshot = capture(CaptureProfile::EmptyTest);

    assert_eq!(snapshot.schema, SchemaIdentifier::current());
    assert_eq!(snapshot.schema.name, "envshot.snapshot");
    assert_eq!(snapshot.schema.major, 0);
    assert_eq!(snapshot.schema.minor, 1);

    assert_eq!(snapshot.producer.name, "envshot");
    assert!(
        !snapshot.producer.version.is_empty(),
        "producer version must be present"
    );

    assert!(snapshot.sections.is_empty());
}

#[test]
fn serializing_the_same_snapshot_twice_produces_equivalent_content() {
    let snapshot = capture(CaptureProfile::EmptyTest);

    let first = serialize(&snapshot);
    let second = serialize(&snapshot);

    assert_eq!(first, second);

    assert_eq!(
        first,
        r#"{"schema":{"name":"envshot.snapshot","major":0,"minor":1},"producer":{"name":"envshot","version":"1.0.0"},"capture":{"profile":"empty_test","privacy":"standard"},"sections":{}}"#
    );
}

#[test]
fn repeated_captures_serialize_identically() {
    let reference = serialize(&capture(CaptureProfile::EmptyTest));

    for _ in 0..8 {
        assert_eq!(serialize(&capture(CaptureProfile::EmptyTest)), reference);
    }
}

#[test]
fn serialized_snapshot_round_trips_without_change() {
    let snapshot = capture(CaptureProfile::EmptyTest);
    let first = serialize(&snapshot);

    let parsed: envshot_model::Snapshot =
        serde_json::from_str(&first).expect("own serialization must parse");
    let second = serialize(&parsed);

    assert_eq!(first, second);
    assert_eq!(parsed, snapshot);
}
