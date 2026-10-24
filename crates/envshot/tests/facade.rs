use envshot::capture_with_privacy;
use envshot_core::{PrivacyPolicy, apply_privacy, normalize};
use envshot_model::{CaptureProfile, LocaleData, PrivacyProfile, SectionData, SectionStatus};

#[test]
fn standard_policy_is_the_default_and_keeps_facts() {
    let snapshot = envshot::capture(CaptureProfile::Minimal);
    assert_eq!(snapshot.capture.privacy, PrivacyProfile::Standard);
    let os = snapshot.sections.get("os").expect("os section");
    assert_eq!(os.status(), SectionStatus::Available);
}

#[test]
fn strict_policy_omits_locale_identifiers() {
    let snapshot = capture_with_privacy(CaptureProfile::Minimal, &PrivacyPolicy::strict());
    assert_eq!(snapshot.capture.privacy, PrivacyProfile::Strict);

    let locale = snapshot.sections.get("locale").expect("locale section");
    assert_eq!(locale.status(), SectionStatus::Unavailable);
    assert_eq!(
        locale.issues()[0].code.as_str(),
        envshot_core::OMITTED_BY_PRIVACY_POLICY
    );
    assert!(locale.data().is_none());

    let os = snapshot.sections.get("os").expect("os section");
    assert_eq!(os.status(), SectionStatus::Available);
}

#[test]
fn off_policy_keeps_data_and_details() {
    let snapshot = capture_with_privacy(CaptureProfile::Minimal, &PrivacyPolicy::off());
    assert_eq!(snapshot.capture.privacy, PrivacyProfile::Off);
    let locale = snapshot.sections.get("locale").expect("locale section");
    assert!(matches!(
        locale.data(),
        Some(SectionData::Locale(LocaleData { .. }))
    ));
}

#[test]
fn privacy_projection_is_idempotent_for_every_policy() {
    let raw = minimal_off_snapshot();
    for policy in [
        PrivacyPolicy::standard(),
        PrivacyPolicy::strict(),
        PrivacyPolicy::off(),
    ] {
        let once = apply_privacy(&raw, &policy);
        let twice = apply_privacy(&once, &policy);
        assert_eq!(
            serde_json::to_string(&once).expect("must serialize"),
            serde_json::to_string(&twice).expect("must serialize"),
            "policy {policy:?} must be idempotent"
        );
    }
}

fn minimal_off_snapshot() -> envshot_model::Snapshot {
    let raw = envshot::capture_with_privacy(CaptureProfile::Minimal, &PrivacyPolicy::off());
    normalize(&raw)
}

#[test]
fn detail_handling_follows_the_policy() {
    let mut snapshot =
        envshot::capture_with_privacy(CaptureProfile::Minimal, &PrivacyPolicy::off());
    let issue = envshot_model::CollectorIssue::new(
        envshot_model::IssueCode::new("parse_failed").expect("static"),
    )
    .with_detail(r"  raw debug: C:\Users\someone\file  ");
    let section = envshot_model::Section::<SectionData>::failed(vec![issue]);
    snapshot.sections.insert("future".to_owned(), section);

    let strict = apply_privacy(&snapshot, &PrivacyPolicy::strict());
    let projected = strict.sections.get("future").expect("section");
    assert_eq!(projected.status(), SectionStatus::Failed);
    assert!(
        projected.issues()[0].detail.is_none(),
        "strict strips details"
    );

    let standard = apply_privacy(&snapshot, &PrivacyPolicy::standard());
    let projected = standard.sections.get("future").expect("section");
    assert!(
        projected.issues()[0].detail.is_none(),
        "standard strips details"
    );

    let off = apply_privacy(&snapshot, &PrivacyPolicy::off());
    let projected = off.sections.get("future").expect("section");
    assert_eq!(
        projected.issues()[0].detail.as_deref(),
        Some(r"  raw debug: C:\Users\someone\file  ")
    );
}

#[test]
fn policy_profile_labels_round_trip() {
    assert_eq!(
        PrivacyPolicy::standard().profile(),
        PrivacyProfile::Standard
    );
    assert_eq!(PrivacyPolicy::strict().profile(), PrivacyProfile::Strict);
    assert_eq!(PrivacyPolicy::off().profile(), PrivacyProfile::Off);
    assert_eq!(
        PrivacyPolicy::from_profile(PrivacyProfile::Strict),
        PrivacyPolicy::strict()
    );
}
