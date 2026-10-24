use envshot_model::{CollectorIssue, IssueCode, SectionStatus, WindowsData};

use crate::CaptureContext;
use crate::collector::{Collector, CollectorDescriptor, CollectorError, SectionContribution};

pub(crate) const CURRENT_VERSION_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";

pub(crate) fn contribution_from_registry(
    edition: Option<String>,
    display_version: Option<String>,
    ubr: Option<u32>,
) -> SectionContribution {
    let data = WindowsData {
        edition: edition.filter(|value| !value.is_empty()),
        display_version: display_version.filter(|value| !value.is_empty()),
        ubr,
    };
    let present = u8::from(data.edition.is_some())
        + u8::from(data.display_version.is_some())
        + u8::from(data.ubr.is_some());
    let status = match present {
        3 => SectionStatus::Available,
        1 | 2 => SectionStatus::Partial,
        _ => {
            let issue = CollectorIssue::new(
                IssueCode::new("windows_version_unavailable").expect("static code"),
            );
            return SectionContribution::new(
                "windows",
                SectionStatus::Unavailable,
                None,
                vec![issue],
            );
        }
    };
    SectionContribution::new(
        "windows",
        status,
        Some(envshot_model::SectionData::Windows(data)),
        Vec::new(),
    )
}

pub(crate) struct WindowsVersionCollector;

impl Collector for WindowsVersionCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor::new(
            "windows.version",
            "windows",
            1,
            envshot_model::SourceKind::SystemFile,
        )
    }

    fn collect(&self, ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
        if !cfg!(windows) {
            let issue = CollectorIssue::new(
                IssueCode::new("windows_version_unsupported").expect("static code"),
            );
            return Ok(SectionContribution::new(
                "windows",
                SectionStatus::Unsupported,
                None,
                vec![issue],
            ));
        }

        let reader = ctx.registry_reader.as_ref();
        let edition = reader.read_string(CURRENT_VERSION_KEY, "EditionID")?;
        let display_version = reader.read_string(CURRENT_VERSION_KEY, "DisplayVersion")?;
        let ubr = reader.read_u32(CURRENT_VERSION_KEY, "UBR")?;
        Ok(contribution_from_registry(edition, display_version, ubr))
    }
}

#[cfg(test)]
mod mapping_tests {
    use super::*;

    #[test]
    fn all_facts_present_is_available() {
        let contribution = contribution_from_registry(
            Some("Professional".to_owned()),
            Some("24H2".to_owned()),
            Some(3031),
        );
        assert_eq!(contribution.status, SectionStatus::Available);
        let Some(envshot_model::SectionData::Windows(data)) = contribution.data else {
            panic!("must carry WindowsData");
        };
        assert_eq!(data.edition.as_deref(), Some("Professional"));
        assert_eq!(data.display_version.as_deref(), Some("24H2"));
        assert_eq!(data.ubr, Some(3031));
    }

    #[test]
    fn some_facts_present_is_partial() {
        let contribution = contribution_from_registry(Some("Professional".to_owned()), None, None);
        assert_eq!(contribution.status, SectionStatus::Partial);
        assert!(
            contribution.issues.is_empty(),
            "partial carries data, not failure issues"
        );
    }

    #[test]
    fn empty_strings_count_as_absent() {
        let contribution =
            contribution_from_registry(Some(String::new()), Some("24H2".to_owned()), Some(3031));
        assert_eq!(contribution.status, SectionStatus::Partial);
        let Some(envshot_model::SectionData::Windows(data)) = contribution.data else {
            panic!("must carry WindowsData");
        };
        assert_eq!(data.edition, None);
    }

    #[test]
    fn no_facts_is_unavailable_with_the_stable_issue() {
        let contribution = contribution_from_registry(None, None, None);
        assert_eq!(contribution.status, SectionStatus::Unavailable);
        assert_eq!(
            contribution.issues[0].code.as_str(),
            "windows_version_unavailable"
        );
    }
}
