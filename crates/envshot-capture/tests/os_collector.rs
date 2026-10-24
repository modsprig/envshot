use envshot_capture::{CollectorRegistry, capture, capture_with_registry};
use envshot_model::{CaptureProfile, OsFamily, SectionData, SectionStatus};

#[test]
fn real_host_os_identity_satisfies_invariants() {
    let snapshot = capture(CaptureProfile::Standard);

    let os = snapshot
        .sections
        .get("os")
        .expect("built-in registry provides the os section");
    assert_eq!(
        os.status(),
        SectionStatus::Available,
        "real host has OS identity"
    );

    let Some(SectionData::Os(os_data)) = os.data() else {
        panic!("os section must carry OsData");
    };
    assert!(
        matches!(
            os_data.family,
            OsFamily::Windows | OsFamily::Linux | OsFamily::MacOs | OsFamily::Unknown
        ),
        "family must be a known variant"
    );
    assert!(
        os_data.version.raw().len() > 3,
        "raw version text must be present, got {:?}",
        os_data.version.raw()
    );
    assert_eq!(os.provenance()[0].collector_id(), "os.host");
    assert_eq!(os.provenance()[0].revision(), 1);
    assert_eq!(
        os.provenance()[0].source_kind(),
        envshot_model::SourceKind::LibraryAdapter
    );
}

#[test]
fn empty_test_profile_still_yields_envelope() {
    let snapshot = capture(CaptureProfile::EmptyTest);
    assert!(snapshot.sections.is_empty());
}

#[test]
fn empty_registry_with_standard_profile_has_no_sections() {
    let snapshot = capture_with_registry(&CollectorRegistry::empty(), CaptureProfile::Standard);
    assert!(snapshot.sections.is_empty());
}

#[test]
fn repeated_standard_captures_are_stable() {
    let first = capture(CaptureProfile::Standard);
    let second = capture(CaptureProfile::Standard);
    assert_eq!(
        serde_json::to_string(&first).expect("must serialize"),
        serde_json::to_string(&second).expect("must serialize"),
        "os identity must be stable between captures"
    );
}
