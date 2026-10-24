mod diff;
mod environment;
mod fingerprint;
mod paths;
mod privacy;
mod render;
mod sanitize;

pub use diff::{Change, ChangeKind, DiffPolicy, Relevance, SnapshotDiff, diff};
pub use environment::{DEFAULT_VALUE_LIMIT, EnvironmentPolicy, bounded_value};
pub use fingerprint::{Fingerprint, FingerprintError, FingerprintProfile, fingerprint};
pub use paths::symbolic_path;
pub use privacy::{OMITTED_BY_PRIVACY_POLICY, PolicyMode, PrivacyPolicy, apply_privacy};
pub use render::{DIFF_SCHEMA_NAME, DIFF_SCHEMA_VERSION, render_json, render_text};
pub use sanitize::{DEFAULT_DETAIL_LIMIT, sanitized_detail};

use std::collections::BTreeMap;

use envshot_model::{
    CollectorIssue, CpuData, LocaleData, OsData, Section, SectionData, Snapshot, VersionValue,
};

#[must_use]
pub fn normalize(snapshot: &Snapshot) -> Snapshot {
    let sections = snapshot
        .sections
        .iter()
        .map(|(id, section)| (id.clone(), normalize_section(section)))
        .collect::<BTreeMap<_, _>>();

    Snapshot {
        schema: snapshot.schema.clone(),
        producer: snapshot.producer.clone(),
        capture: snapshot.capture.clone(),
        sections,
    }
}

fn normalize_section(section: &Section<SectionData>) -> Section<SectionData> {
    let data = section.data().map(normalize_data);
    let issues = section
        .issues()
        .iter()
        .map(|issue| CollectorIssue {
            code: issue.code.clone(),
            detail: issue.detail.as_deref().map(str::trim).map(str::to_owned),
        })
        .collect();
    Section::new(section.status(), data, issues)
        .expect("normalization keeps a valid status/data combination")
        .with_provenance(section.provenance().to_vec())
}

fn normalize_data(data: &SectionData) -> SectionData {
    match data {
        SectionData::Os(os) => SectionData::Os(OsData {
            family: os.family,
            version: VersionValue::new(os.version.raw()),
            build: os
                .build
                .as_ref()
                .map(|build| VersionValue::new(build.raw())),
        }),
        SectionData::Cpu(cpu) => SectionData::Cpu(CpuData {
            model: cpu.model.as_deref().map(collapse_whitespace),
            logical_count: cpu.logical_count,
            physical_count: cpu.physical_count,
            features: cpu
                .features
                .iter()
                .map(|feature| feature.trim().to_lowercase())
                .collect(),
        }),
        SectionData::Locale(locale) => SectionData::Locale(LocaleData {
            languages: locale
                .languages
                .iter()
                .map(|identifier| identifier.trim().to_owned())
                .collect(),
            timezone: locale.timezone.as_deref().map(str::trim).map(str::to_owned),
        }),
        SectionData::Gpu(adapters) => SectionData::Gpu(
            adapters
                .iter()
                .map(|adapter| envshot_model::GpuData {
                    index: adapter.index,
                    name: adapter.name.trim().to_owned(),
                    vendor_id: adapter.vendor_id,
                    device_id: adapter.device_id,
                    driver_version: adapter
                        .driver_version
                        .as_ref()
                        .map(|version| VersionValue::new(version.raw())),
                })
                .collect(),
        ),
        SectionData::Distro(distro) => SectionData::Distro(envshot_model::DistroData {
            name: distro.name.as_deref().map(str::trim).map(str::to_owned),
            version: distro
                .version
                .as_ref()
                .map(|version| VersionValue::new(version.raw())),
        }),
        SectionData::Windows(windows) => SectionData::Windows(envshot_model::WindowsData {
            edition: windows.edition.as_deref().map(str::trim).map(str::to_owned),
            display_version: windows
                .display_version
                .as_deref()
                .map(str::trim)
                .map(str::to_owned),
            ubr: windows.ubr,
        }),
        other => other.clone(),
    }
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_runs_collapse_to_single_spaces() {
        assert_eq!(collapse_whitespace("Fixture\t CPU \t X3"), "Fixture CPU X3");
        assert_eq!(collapse_whitespace("  trimmed  "), "trimmed");
        assert_eq!(collapse_whitespace(""), "");
    }
}

#[doc(hidden)]
pub mod testing {

    #[must_use]
    pub fn canaries() -> Vec<String> {
        [
            "S3CR3T_CANARY_USERNAME",
            "S3CR3T_CANARY_HOSTNAME",
            "S3CR3T_CANARY_HOMEPATH",
            "S3CR3T_CANARY_IP_ADDRESS",
            "S3CR3T_CANARY_MAC_ADDRESS",
            "S3CR3T_CANARY_SERIAL",
            "S3CR3T_CANARY_MACHINE_UUID",
            "S3CR3T_CANARY_TOKEN",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn assert_no_canaries(serialized: &str) {
        for canary in canaries() {
            assert!(
                !serialized.contains(canary.as_str()),
                "canary leaked into output: {canary}"
            );
        }
    }
}

#[cfg(test)]
mod testing_tests {
    use super::testing::*;

    #[test]
    fn canaries_cover_the_identifier_classes_with_the_deterministic_prefix() {
        let canaries = canaries();
        assert_eq!(canaries.len(), 8);
        assert!(canaries.iter().all(|c| c.starts_with("S3CR3T_CANARY_")));
        assert!(canaries.iter().any(|c| c.contains("USERNAME")));
        assert!(canaries.iter().any(|c| c.contains("TOKEN")));
        assert!(canaries.iter().all(|c| !c.is_empty()));
    }

    #[test]
    fn assert_no_canaries_passes_on_clean_output_and_panics_on_leaks() {
        assert_no_canaries("clean standard output");
        let leaked = String::from("{\"detail\":\"S3CR3T_CANARY_TOKEN\"}");
        let result = std::panic::catch_unwind(|| assert_no_canaries(&leaked));
        assert!(result.is_err(), "the scan must detect leaks");
    }
}
