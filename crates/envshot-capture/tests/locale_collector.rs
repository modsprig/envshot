use envshot_capture::capture;
use envshot_model::{CaptureProfile, SectionData, SectionStatus};

#[test]
fn real_host_locale_satisfies_invariants() {
    let snapshot = capture(CaptureProfile::Standard);

    let section = snapshot
        .sections
        .get("locale")
        .expect("built-in locale section");
    assert!(matches!(
        section.status(),
        SectionStatus::Available | SectionStatus::Partial
    ));
    let Some(SectionData::Locale(data)) = section.data() else {
        panic!("partial/available locale section must carry LocaleData");
    };
    if let Some(timezone) = &data.timezone {
        assert!(
            !timezone.is_empty(),
            "timezone identifier must be non-empty"
        );
    }
    assert_eq!(section.provenance()[0].collector_id(), "locale.host");
}

#[test]
fn empty_test_profile_still_yields_envelope() {
    let snapshot = capture(CaptureProfile::EmptyTest);
    assert!(snapshot.sections.is_empty());
}
