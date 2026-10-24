use envshot_capture::capture;
use envshot_model::{CaptureProfile, SectionData, SectionStatus, VirtualizationCategory};

#[test]
fn real_host_virtualization_satisfies_invariants() {
    let snapshot = capture(CaptureProfile::Standard);

    let section = snapshot
        .sections
        .get("virtualization")
        .expect("built-in virtualization section");
    assert_eq!(section.status(), SectionStatus::Available);
    let Some(SectionData::Virtualization(data)) = section.data() else {
        panic!("virtualization section must carry VirtualizationData");
    };
    assert!(
        matches!(
            data.category,
            VirtualizationCategory::Physical
                | VirtualizationCategory::VirtualMachine
                | VirtualizationCategory::Container
                | VirtualizationCategory::Unknown
        ),
        "category must be a known variant"
    );
    assert_eq!(
        section.provenance()[0].collector_id(),
        "virtualization.host"
    );
}

#[test]
fn minimal_profile_produces_the_mvp_sections() {
    let snapshot = capture(CaptureProfile::Minimal);

    let names: Vec<_> = snapshot.sections.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            "architecture",
            "cpu",
            "distro",
            "gpu",
            "locale",
            "memory",
            "os",
            "virtualization",
            "windows"
        ]
    );
    for section in snapshot.sections.values() {
        assert!(
            matches!(
                section.status(),
                SectionStatus::Available
                    | SectionStatus::Partial
                    | SectionStatus::Unavailable
                    | SectionStatus::Unsupported
            ),
            "built-in collectors must not fail on a real host"
        );
    }
}

#[test]
fn empty_test_profile_still_yields_envelope() {
    let snapshot = capture(CaptureProfile::EmptyTest);
    assert!(snapshot.sections.is_empty());
}
