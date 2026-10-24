use envshot_model::{
    CollectorIssue, IssueCode, PrivacyProfile, Section, SectionData, SectionStatus, Snapshot,
};

use crate::normalize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivacyPolicy {
    mode: PolicyMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyMode {
    Standard,

    Strict,

    Off,
}

impl PrivacyPolicy {
    #[must_use]
    pub fn standard() -> Self {
        Self {
            mode: PolicyMode::Standard,
        }
    }

    #[must_use]
    pub fn strict() -> Self {
        Self {
            mode: PolicyMode::Strict,
        }
    }

    #[must_use]
    pub fn off() -> Self {
        Self {
            mode: PolicyMode::Off,
        }
    }

    #[must_use]
    pub fn from_profile(profile: PrivacyProfile) -> Self {
        let mode = match profile {
            PrivacyProfile::Standard => PolicyMode::Standard,
            PrivacyProfile::Strict => PolicyMode::Strict,
            PrivacyProfile::Off => PolicyMode::Off,
        };
        Self { mode }
    }

    #[must_use]
    pub fn profile(&self) -> PrivacyProfile {
        match self.mode {
            PolicyMode::Standard => PrivacyProfile::Standard,
            PolicyMode::Strict => PrivacyProfile::Strict,
            PolicyMode::Off => PrivacyProfile::Off,
        }
    }
}

#[must_use]
pub fn apply_privacy(snapshot: &Snapshot, policy: &PrivacyPolicy) -> Snapshot {
    if policy.mode == PolicyMode::Off {
        let mut as_captured = snapshot.clone();
        as_captured.capture.privacy = policy.profile();
        return as_captured;
    }

    let mut normalized = normalize(snapshot);
    normalized.capture.privacy = policy.profile();

    let sections = normalized
        .sections
        .into_iter()
        .map(|(id, section)| (id, project_section(&section, *policy)))
        .collect();
    Snapshot {
        sections,
        ..normalized
    }
}

fn project_section(section: &Section<SectionData>, policy: PrivacyPolicy) -> Section<SectionData> {
    if policy.mode == PolicyMode::Strict && matches!(section.data(), Some(SectionData::Locale(_))) {
        let issues = vec![omitted_issue()];
        return Section::new(SectionStatus::Unavailable, None, issues)
            .expect("unavailable without data is valid")
            .with_provenance(section.provenance().to_vec());
    }

    let data = section.data().cloned();

    let issues: Vec<CollectorIssue> = section
        .issues()
        .iter()
        .map(|issue| CollectorIssue {
            code: issue.code.clone(),
            detail: None,
        })
        .collect();
    Section::new(section.status(), data, issues)
        .expect("projection keeps a valid status/data combination")
        .with_provenance(section.provenance().to_vec())
}

pub const OMITTED_BY_PRIVACY_POLICY: &str = "omitted_by_privacy_policy";

pub(crate) fn omitted_issue() -> CollectorIssue {
    CollectorIssue::new(IssueCode::new(OMITTED_BY_PRIVACY_POLICY).expect("static code"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use envshot_model::{
        CaptureMetadata, CaptureProfile, LocaleData, ProducerInfo, SchemaIdentifier,
    };

    fn base() -> Snapshot {
        Snapshot {
            schema: SchemaIdentifier::current(),
            producer: ProducerInfo {
                name: "envshot".to_owned(),
                version: "0.0.0".to_owned(),
            },
            capture: CaptureMetadata {
                profile: CaptureProfile::Standard,
                privacy: PrivacyProfile::Standard,
            },
            sections: std::collections::BTreeMap::new(),
        }
    }

    fn snapshot_with_locale_and_detail() -> Snapshot {
        let mut snapshot = base();
        let locale = Section::new(
            SectionStatus::Available,
            Some(SectionData::Locale(LocaleData {
                languages: vec!["en-US".to_owned()],
                timezone: Some("Etc/UTC".to_owned()),
            })),
            vec![
                CollectorIssue::new(IssueCode::new("parse_failed").expect("static"))
                    .with_detail("raw debug text"),
            ],
        )
        .expect("valid");
        snapshot.sections.insert("locale".to_owned(), locale);
        snapshot
    }

    #[test]
    fn profile_labels_round_trip() {
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

    #[test]
    fn standard_keeps_locale_but_strips_details() {
        let projected = apply_privacy(
            &snapshot_with_locale_and_detail(),
            &PrivacyPolicy::standard(),
        );
        assert_eq!(projected.capture.privacy, PrivacyProfile::Standard);
        let locale = projected.sections.get("locale").expect("locale kept");
        assert_eq!(locale.status(), SectionStatus::Available);
        assert!(
            locale.issues()[0].detail.is_none(),
            "details are a debug channel"
        );
    }

    #[test]
    fn strict_omits_locale_with_the_stable_issue() {
        let projected = apply_privacy(&snapshot_with_locale_and_detail(), &PrivacyPolicy::strict());
        let locale = projected.sections.get("locale").expect("section kept");
        assert_eq!(locale.status(), SectionStatus::Unavailable);
        assert!(locale.data().is_none());
        assert_eq!(locale.issues()[0].code.as_str(), OMITTED_BY_PRIVACY_POLICY);
    }

    #[test]
    fn off_is_an_identity_projection() {
        let snapshot = snapshot_with_locale_and_detail();
        let projected = apply_privacy(&snapshot, &PrivacyPolicy::off());
        assert_eq!(projected.capture.privacy, PrivacyProfile::Off);

        assert_eq!(projected.sections, snapshot.sections);
        assert_eq!(projected.producer, snapshot.producer);
    }

    #[test]
    fn projection_is_idempotent_for_every_policy() {
        let snapshot = snapshot_with_locale_and_detail();
        for policy in [
            PrivacyPolicy::standard(),
            PrivacyPolicy::strict(),
            PrivacyPolicy::off(),
        ] {
            let once = apply_privacy(&snapshot, &policy);
            let twice = apply_privacy(&once, &policy);
            assert_eq!(once, twice);
        }
    }
}
