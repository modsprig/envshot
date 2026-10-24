use envshot_model::{CollectorIssue, DistroData, IssueCode, SectionStatus, VersionValue};

use crate::CaptureContext;
use crate::collector::{Collector, CollectorDescriptor, CollectorError, SectionContribution};

pub(crate) const OS_RELEASE_PATHS: [&str; 2] = ["/etc/os-release", "/usr/lib/os-release"];

pub(crate) fn parse_os_release(content: &str) -> Vec<(String, String)> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| {
            let value = value.trim();
            let unquoted = if value.len() >= 2
                && ((value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\'')))
            {
                &value[1..value.len() - 1]
            } else {
                value
            };
            (key.trim().to_owned(), unquoted.to_owned())
        })
        .collect()
}

pub(crate) fn distro_contribution(
    entries: Result<Vec<(String, String)>, CollectorError>,
) -> SectionContribution {
    #[allow(clippy::manual_let_else)]
    let entries = match entries {
        Ok(entries) => entries,
        Err(_) => {
            return SectionContribution::new(
                "distro",
                SectionStatus::Unavailable,
                None,
                vec![CollectorIssue::new(
                    IssueCode::new("distro_unavailable").expect("static code"),
                )],
            );
        }
    };
    let lookup = |key: &str| -> Option<String> {
        entries
            .iter()
            .rev()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.clone())
    };
    let name = lookup("NAME").filter(|value| !value.is_empty());
    let version = lookup("VERSION_ID")
        .filter(|value| !value.is_empty())
        .map(VersionValue::new);

    match (name, version) {
        (None, None) => SectionContribution::new(
            "distro",
            SectionStatus::Unavailable,
            None,
            vec![CollectorIssue::new(
                IssueCode::new("distro_unavailable").expect("static code"),
            )],
        ),
        (name, version) => SectionContribution::new(
            "distro",
            SectionStatus::Available,
            Some(envshot_model::SectionData::Distro(DistroData {
                name,
                version,
            })),
            Vec::new(),
        ),
    }
}

pub(crate) struct DistroCollector;

impl Collector for DistroCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor::new(
            "distro.host",
            "distro",
            1,
            envshot_model::SourceKind::SystemFile,
        )
    }

    fn collect(&self, _ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
        if !cfg!(target_os = "linux") {
            let issue =
                CollectorIssue::new(IssueCode::new("distro_unsupported").expect("static code"));
            return Ok(SectionContribution::new(
                "distro",
                SectionStatus::Unsupported,
                None,
                vec![issue],
            ));
        }

        let entries = read_os_release();
        Ok(distro_contribution(entries))
    }
}

fn read_os_release() -> Result<Vec<(String, String)>, CollectorError> {
    for path in OS_RELEASE_PATHS {
        if let Ok(content) = std::fs::read_to_string(path) {
            return Ok(parse_os_release(&content));
        }
    }
    Err(CollectorError::new("no os-release file"))
}

#[cfg(test)]
mod mapping_tests {
    use super::*;

    #[test]
    fn parses_quoted_and_unquoted_values_with_comments() {
        let entries =
            parse_os_release("# a comment\nNAME=\"Fedora Linux\"\nVERSION_ID=42\n\nID=fedora\n");
        assert!(entries.contains(&("NAME".to_owned(), "Fedora Linux".to_owned())));
        assert!(entries.contains(&("VERSION_ID".to_owned(), "42".to_owned())));
        assert!(entries.contains(&("ID".to_owned(), "fedora".to_owned())));
        assert_eq!(entries.len(), 3, "comments and blank lines are skipped");
    }

    #[test]
    fn single_quoted_values_are_unquoted() {
        let entries = parse_os_release("NAME='Test OS'\n");
        assert!(entries.contains(&("NAME".to_owned(), "Test OS".to_owned())));
    }

    #[test]
    fn both_values_present_is_available() {
        let contribution = distro_contribution(Ok(vec![
            ("NAME".to_owned(), "Fedora Linux".to_owned()),
            ("VERSION_ID".to_owned(), "42".to_owned()),
        ]));
        assert_eq!(contribution.status, SectionStatus::Available);
        let Some(envshot_model::SectionData::Distro(data)) = contribution.data else {
            panic!("must carry DistroData");
        };
        assert_eq!(data.name.as_deref(), Some("Fedora Linux"));
        assert_eq!(
            data.version.as_ref().map(envshot_model::VersionValue::raw),
            Some("42")
        );
    }

    #[test]
    fn nothing_present_is_unavailable_with_the_stable_issue() {
        let contribution = distro_contribution(Ok(vec![("ID".to_owned(), "fedora".to_owned())]));
        assert_eq!(contribution.status, SectionStatus::Unavailable);
        assert_eq!(contribution.issues[0].code.as_str(), "distro_unavailable");
    }

    #[test]
    fn unreadable_file_is_unavailable() {
        let contribution = distro_contribution(Err(CollectorError::new("hostile")));
        assert_eq!(contribution.status, SectionStatus::Unavailable);
    }
}
