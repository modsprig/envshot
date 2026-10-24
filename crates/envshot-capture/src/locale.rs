use envshot_model::{CollectorIssue, IssueCode, LocaleData, SectionStatus};

use crate::CaptureContext;
use crate::collector::{Collector, CollectorDescriptor, CollectorError, SectionContribution};

pub(crate) fn locale_contribution(
    locale: Option<String>,
    timezone: Option<String>,
) -> SectionContribution {
    let issue = |code: &str| CollectorIssue::new(IssueCode::new(code).expect("static code"));
    match (locale, timezone) {
        (Some(identifier), Some(tz)) => SectionContribution::new(
            "locale",
            SectionStatus::Available,
            Some(envshot_model::SectionData::Locale(LocaleData {
                languages: vec![identifier],
                timezone: Some(tz),
            })),
            Vec::new(),
        ),
        (Some(identifier), None) => SectionContribution::new(
            "locale",
            SectionStatus::Partial,
            Some(envshot_model::SectionData::Locale(LocaleData {
                languages: vec![identifier],
                timezone: None,
            })),
            vec![issue("timezone_unavailable")],
        ),
        (None, Some(tz)) => SectionContribution::new(
            "locale",
            SectionStatus::Partial,
            Some(envshot_model::SectionData::Locale(LocaleData {
                languages: Vec::new(),
                timezone: Some(tz),
            })),
            vec![issue("locale_language_unavailable")],
        ),
        (None, None) => SectionContribution::new(
            "locale",
            SectionStatus::Unavailable,
            None,
            vec![issue("locale_unavailable")],
        ),
    }
}

pub(crate) struct LocaleCollector;

impl Collector for LocaleCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor::new(
            "locale.host",
            "locale",
            1,
            envshot_model::SourceKind::LibraryAdapter,
        )
    }

    fn collect(&self, _ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
        let locale = sys_locale::get_locale();
        let timezone = iana_time_zone::get_timezone().ok();
        Ok(locale_contribution(locale, timezone))
    }
}

#[cfg(test)]
mod mapping_tests {
    use super::*;

    fn codes(contribution: &SectionContribution) -> Vec<&str> {
        contribution
            .issues
            .iter()
            .map(|i| i.code.as_str())
            .collect()
    }

    #[test]
    fn both_facts_present_is_available() {
        let contribution =
            locale_contribution(Some("en-US".to_owned()), Some("Europe/Berlin".to_owned()));
        assert_eq!(contribution.status, SectionStatus::Available);
        assert!(contribution.issues.is_empty());
        let Some(envshot_model::SectionData::Locale(data)) = contribution.data else {
            panic!("must carry LocaleData");
        };
        assert_eq!(data.languages, ["en-US"]);
        assert_eq!(data.timezone.as_deref(), Some("Europe/Berlin"));
    }

    #[test]
    fn missing_timezone_is_partial_with_the_stable_issue() {
        let contribution = locale_contribution(Some("en-US".to_owned()), None);
        assert_eq!(contribution.status, SectionStatus::Partial);
        assert_eq!(codes(&contribution), ["timezone_unavailable"]);
        assert!(contribution.data.is_some(), "partial requires data");
    }

    #[test]
    fn missing_locale_is_partial_with_the_stable_issue() {
        let contribution = locale_contribution(None, Some("Europe/Berlin".to_owned()));
        assert_eq!(contribution.status, SectionStatus::Partial);
        assert_eq!(codes(&contribution), ["locale_language_unavailable"]);
    }

    #[test]
    fn missing_both_is_unavailable_without_data() {
        let contribution = locale_contribution(None, None);
        assert_eq!(contribution.status, SectionStatus::Unavailable);
        assert_eq!(codes(&contribution), ["locale_unavailable"]);
        assert!(contribution.data.is_none());
    }
}
