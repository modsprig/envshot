use envshot_model::{CollectorIssue, IssueCode, OsData, OsFamily, SectionStatus, VersionValue};

use crate::CaptureContext;
use crate::collector::{Collector, CollectorDescriptor, CollectorError, SectionContribution};

pub(crate) fn map_os_identity(
    platform: &str,
    os_version: Option<&str>,
    kernel_version: Option<&str>,
) -> (OsFamily, Option<VersionValue>, Option<VersionValue>) {
    let family = family_for(platform);
    let version = os_version.map(VersionValue::new);
    let build = build_for(family, kernel_version);
    (family, version, build)
}

fn family_for(platform: &str) -> OsFamily {
    match platform {
        "windows" => OsFamily::Windows,
        "linux" => OsFamily::Linux,
        "macos" => OsFamily::MacOs,
        _ => OsFamily::Unknown,
    }
}

fn build_for(family: OsFamily, kernel_version: Option<&str>) -> Option<VersionValue> {
    match family {
        OsFamily::Windows => kernel_version.map(VersionValue::new),
        _ => None,
    }
}

pub(crate) struct OsCollector;

impl Collector for OsCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor::new(
            "os.host",
            "os",
            1,
            envshot_model::SourceKind::LibraryAdapter,
        )
    }

    fn collect(&self, _ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
        let (family, version, build) = map_os_identity(
            std::env::consts::OS,
            sysinfo::System::os_version().as_deref(),
            sysinfo::System::kernel_version().as_deref(),
        );
        Ok(contribution_from_identity(family, version, build))
    }
}

pub(crate) fn contribution_from_identity(
    family: OsFamily,
    version: Option<VersionValue>,
    build: Option<VersionValue>,
) -> SectionContribution {
    let Some(version) = version else {
        let issue =
            CollectorIssue::new(IssueCode::new("os_identity_unavailable").expect("static code"));
        return SectionContribution::new("os", SectionStatus::Unavailable, None, vec![issue]);
    };

    SectionContribution::new(
        "os",
        SectionStatus::Available,
        Some(envshot_model::SectionData::Os(OsData {
            family,
            version,
            build,
        })),
        Vec::new(),
    )
}

#[cfg(test)]
mod mapping_tests {
    use super::*;

    #[test]
    fn windows_identity_strings_map_to_os_data() {
        let (family, version, build) =
            map_os_identity("windows", Some("10.0.26100"), Some("26100"));

        assert_eq!(family, OsFamily::Windows);
        let version = version.expect("version string present");
        assert_eq!(version.raw(), "10.0.26100");
        assert_eq!(
            version.normalized(),
            Some(&envshot_model::NormalizedVersion {
                major: 10,
                minor: Some(0),
                patch: Some(26100),
            })
        );
        assert_eq!(build.expect("build string present").raw(), "26100");
    }

    #[test]
    fn linux_kernel_string_is_not_treated_as_an_os_build() {
        let (family, version, build) =
            map_os_identity("linux", Some("6.9.12"), Some("6.9.12-300.fc40.x86_64"));

        assert_eq!(family, OsFamily::Linux);
        assert_eq!(version.expect("version present").raw(), "6.9.12");
        assert!(build.is_none(), "kernel strings are not OS builds");
    }

    #[test]
    fn unknown_platform_maps_to_unknown_family() {
        let (family, _, _) = map_os_identity("solaris", Some("11.4"), None);
        assert_eq!(family, OsFamily::Unknown);
    }

    #[test]
    fn absent_strings_map_to_none() {
        let (family, version, build) = map_os_identity("windows", None, None);
        assert_eq!(family, OsFamily::Windows);
        assert!(version.is_none());
        assert!(build.is_none());
    }

    #[test]
    fn macos_platform_maps_to_macos_family() {
        let (family, version, build) = map_os_identity("macos", Some("15.0"), Some("24A335"));
        assert_eq!(family, OsFamily::MacOs);
        assert_eq!(version.expect("version present").raw(), "15.0");
        assert!(build.is_none(), "kernel strings are not OS builds");
    }

    #[test]
    fn identity_with_version_but_without_build_is_available() {
        let contribution =
            contribution_from_identity(OsFamily::Linux, Some(VersionValue::new("6.9")), None);
        assert_eq!(contribution.status, SectionStatus::Available);
    }

    #[test]
    fn identity_with_build_but_without_version_is_unavailable() {
        let contribution =
            contribution_from_identity(OsFamily::Windows, None, Some(VersionValue::new("26100")));
        assert_eq!(contribution.status, SectionStatus::Unavailable);
        assert_eq!(
            contribution.issues[0].code.as_str(),
            "os_identity_unavailable"
        );
    }

    #[test]
    fn identity_without_any_string_is_unavailable_with_the_stable_issue() {
        let contribution = contribution_from_identity(OsFamily::Windows, None, None);
        assert_eq!(contribution.status, SectionStatus::Unavailable);
        assert_eq!(
            contribution.issues[0].code.as_str(),
            "os_identity_unavailable"
        );
    }
}
