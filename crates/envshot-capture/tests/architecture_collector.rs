use envshot_capture::{CollectorRegistry, capture, capture_with_registry};
use envshot_model::{Architecture, CaptureProfile, SectionData, SectionStatus};

#[test]
fn real_host_architecture_satisfies_invariants() {
    let snapshot = capture(CaptureProfile::Standard);

    let section = snapshot
        .sections
        .get("architecture")
        .expect("built-in registry provides the architecture section");
    assert_eq!(section.status(), SectionStatus::Available);

    let Some(SectionData::Architecture(arch)) = section.data() else {
        panic!("architecture section must carry ArchitectureData");
    };

    assert_ne!(
        arch.process,
        Architecture::Unknown,
        "process arch is std-known"
    );
    assert!(
        matches!(
            arch.os,
            Architecture::X86 | Architecture::X64 | Architecture::Arm64
        ),
        "a healthy Windows host always reports its real native machine"
    );
    assert_eq!(
        arch.process, arch.os,
        "this 64-bit native (non-WOW64, non-emulated) build must agree"
    );
    assert_eq!(section.provenance()[0].collector_id(), "architecture.host");
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
        serde_json::to_string(&second).expect("must serialize")
    );
}
