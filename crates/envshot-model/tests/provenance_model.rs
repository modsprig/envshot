use envshot_model::{Provenance, Section, SectionStatus, SourceKind, read_snapshot};

fn provenance(id: &str, revision: u32, kind: SourceKind) -> Provenance {
    Provenance::new(id, revision, kind).expect("test ids are non-empty")
}

#[test]
fn section_records_provenance_and_round_trips() {
    let provenance = provenance("os.windows.version", 1, SourceKind::NativeApi);
    let section = Section::<u32>::new(SectionStatus::Available, Some(7), Vec::new())
        .expect("valid")
        .with_provenance(vec![provenance.clone()]);

    assert_eq!(section.provenance(), std::slice::from_ref(&provenance));

    let json = serde_json::to_string(&section).expect("serialization must not fail");
    assert_eq!(
        json,
        r#"{"status":"available","data":7,"provenance":[{"collector_id":"os.windows.version","revision":1,"source_kind":"native_api","notes":null}],"issues":[]}"#
    );

    let parsed: Section<u32> = serde_json::from_str(&json).expect("own serialization must parse");
    assert_eq!(parsed, section);
}

#[test]
fn provenance_order_is_deterministic_and_deduplicated() {
    let a = provenance("cpu.features", 1, SourceKind::SystemFile);
    let b = provenance("os.version", 2, SourceKind::LibraryAdapter);
    let c = provenance("cpu.features", 1, SourceKind::SystemFile);

    let first = Section::<()>::unsupported(Vec::new()).with_provenance(vec![
        a.clone(),
        b.clone(),
        c.clone(),
    ]);
    let second = Section::<()>::unsupported(Vec::new()).with_provenance(vec![b.clone(), a.clone()]);

    assert_eq!(
        first, second,
        "equal provenance sets must be equal sections"
    );

    let first_json = serde_json::to_string(&first).expect("serialization must not fail");
    let second_json = serde_json::to_string(&second).expect("serialization must not fail");
    assert_eq!(first_json, second_json);

    assert_eq!(first.provenance().len(), 2, "exact duplicate dropped");
    assert_eq!(first.provenance()[0].collector_id(), "cpu.features");
    assert_eq!(first.provenance()[1].collector_id(), "os.version");
}

#[test]
fn empty_collector_id_is_rejected() {
    assert!(Provenance::new("", 1, SourceKind::Derived).is_err());
}

#[test]
fn provenance_notes_round_trip() {
    let provenance =
        provenance("gpu.adapter", 3, SourceKind::LibraryAdapter).with_notes("vendor ids only");
    assert_eq!(provenance.notes(), Some("vendor ids only"));

    let json = serde_json::to_string(&provenance).expect("serialization must not fail");
    let parsed: Provenance = serde_json::from_str(&json).expect("own serialization must parse");
    assert_eq!(parsed, provenance);
}

#[test]
fn reader_preserves_and_normalizes_provenance() {
    let document = r#"{
        "schema": {"name": "envshot.snapshot", "major": 0, "minor": 1},
        "producer": {"name": "envshot", "version": "0.0.0"},
        "capture": {"profile": "empty_test", "privacy": "standard"},
        "sections": {
            "os": {
                "status": "failed",
                "provenance": [
                    {"collector_id": "os.version", "revision": 2, "source_kind": "library_adapter"},
                    {"collector_id": "cpu.features", "revision": 1, "source_kind": "system_file"}
                ]
            }
        }
    }"#;

    let snapshot = read_snapshot(document).expect("valid document must read");
    let os = snapshot.sections.get("os").expect("os section");

    assert_eq!(os.provenance().len(), 2);
    assert_eq!(
        os.provenance()[0].collector_id(),
        "cpu.features",
        "provenance must be normalized (sorted) on read"
    );
    assert_eq!(os.provenance()[1].source_kind(), SourceKind::LibraryAdapter);
}

#[test]
fn reader_rejects_unknown_source_kind() {
    let document = r#"{
        "schema": {"name": "envshot.snapshot", "major": 0, "minor": 1},
        "producer": {"name": "envshot", "version": "0.0.0"},
        "capture": {"profile": "empty_test", "privacy": "standard"},
        "sections": {
            "os": {
                "status": "failed",
                "provenance": [{"collector_id": "os.version", "revision": 1, "source_kind": "telepathy"}]
            }
        }
    }"#;

    let error = read_snapshot(document).expect_err("unknown source kind must be rejected");
    assert_eq!(error.code(), "snapshot.read.malformed");
}

#[test]
fn reader_rejects_non_string_collector_id() {
    let document = r#"{
        "schema": {"name": "envshot.snapshot", "major": 0, "minor": 1},
        "producer": {"name": "envshot", "version": "0.0.0"},
        "capture": {"profile": "empty_test", "privacy": "standard"},
        "sections": {
            "os": {
                "status": "failed",
                "provenance": [{"collector_id": 7, "revision": 1, "source_kind": "derived"}]
            }
        }
    }"#;

    let error = read_snapshot(document).expect_err("non-string id must be rejected");
    assert_eq!(error.code(), "snapshot.read.malformed");
}

#[test]
fn provenance_revision_accessor_reports_recorded_value() {
    assert_eq!(
        provenance("os.version", 0, SourceKind::NativeApi).revision(),
        0
    );
    assert_eq!(
        provenance("os.version", 1, SourceKind::NativeApi).revision(),
        1
    );
    assert_eq!(
        provenance("os.version", u32::MAX, SourceKind::NativeApi).revision(),
        u32::MAX
    );
}
