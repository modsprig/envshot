use serde::Serialize;

use crate::diff::{Change, ChangeKind, SnapshotDiff};

pub const DIFF_SCHEMA_NAME: &str = "envshot.diff";

pub const DIFF_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
struct DiffDocument<'a> {
    diff_schema: DiffSchema,
    changes: &'a [Change],
}

#[derive(Debug, Serialize)]
struct DiffSchema {
    name: &'static str,
    version: u32,
}

#[must_use]
pub fn render_json(diff: &SnapshotDiff) -> String {
    let document = DiffDocument {
        diff_schema: DiffSchema {
            name: DIFF_SCHEMA_NAME,
            version: DIFF_SCHEMA_VERSION,
        },
        changes: diff.changes(),
    };
    serde_json::to_string(&document).expect("diff serialization cannot fail")
}

#[must_use]
pub fn render_text(diff: &SnapshotDiff) -> String {
    if diff.is_empty() {
        return "no changes".to_owned();
    }
    let mut output = String::new();
    let mut current_section: Option<&str> = None;
    for change in diff.changes() {
        let section = change.path.split('.').next().unwrap_or(&change.path);
        if current_section != Some(section) {
            if current_section.is_some() {
                output.push('\n');
            }
            output.push_str(section);
            output.push_str(":\n");
            current_section = Some(section);
        }
        output.push_str("  ");
        output.push_str(&render_change(change));
        output.push('\n');
    }
    output
}

fn render_change(change: &Change) -> String {
    let field = change.path.split('.').skip(1).collect::<Vec<_>>().join(".");
    match change.kind {
        ChangeKind::Added => {
            if field.is_empty() {
                "section added".to_owned()
            } else {
                format!("{field}: added")
            }
        }
        ChangeKind::Removed => {
            if field.is_empty() {
                "section removed".to_owned()
            } else {
                format!("{field}: removed")
            }
        }
        ChangeKind::StatusChanged | ChangeKind::Changed => {
            let before = change.before.as_deref().unwrap_or("<absent>");
            let after = change.after.as_deref().unwrap_or("<absent>");
            format!("{field}: {before} -> {after}")
        }
        ChangeKind::SetChanged => {
            let mut line = format!("{field}:");
            if let Some(added) = change.after.as_deref().filter(|s| !s.is_empty()) {
                line.push_str(" + ");
                line.push_str(added);
                line.push(';');
            }
            if let Some(removed) = change.before.as_deref().filter(|s| !s.is_empty()) {
                line.push_str(" - ");
                line.push_str(removed);
                line.push(';');
            }
            line.trim_end_matches(';').to_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::diff::{DiffPolicy, diff};
    use envshot_model::{
        CaptureMetadata, CaptureProfile, CpuData, LocaleData, PrivacyProfile, ProducerInfo,
        SchemaIdentifier, Section, SectionData, Snapshot,
    };

    fn snapshot_with(sections: Vec<(&'static str, SectionData)>) -> Snapshot {
        let mut map = std::collections::BTreeMap::new();
        for (id, data) in sections {
            map.insert(id.to_owned(), Section::available(data));
        }
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
            sections: map,
        }
    }

    fn sample_diff() -> SnapshotDiff {
        let a = snapshot_with(vec![(
            "cpu",
            SectionData::Cpu(CpuData {
                model: Some("old cpu".to_owned()),
                logical_count: Some(8),
                physical_count: None,
                features: BTreeSet::from(["sse2".to_owned()]),
            }),
        )]);
        let b = snapshot_with(vec![
            (
                "cpu",
                SectionData::Cpu(CpuData {
                    model: Some("new cpu".to_owned()),
                    logical_count: Some(8),
                    physical_count: None,
                    features: BTreeSet::from(["avx512f".to_owned()]),
                }),
            ),
            (
                "locale",
                SectionData::Locale(LocaleData {
                    languages: vec![],
                    timezone: None,
                }),
            ),
        ]);
        diff(&a, &b, &DiffPolicy::diagnostic())
    }

    #[test]
    fn text_rendering_groups_by_section_and_renders_each_kind() {
        let text = render_text(&sample_diff());
        assert!(
            text.contains("cpu:\n  features: + avx512f; - sse2\n  model: old cpu -> new cpu\n"),
            "actual: {text:?}"
        );
        assert!(text.contains("locale:\n  section added\n"));
    }

    #[test]
    fn empty_diffs_render_as_no_changes() {
        let a = snapshot_with(vec![]);
        let diff = diff(&a, &a, &DiffPolicy::compatibility());
        assert_eq!(render_text(&diff), "no changes");
    }

    #[test]
    fn absent_values_render_as_markers() {
        let a = snapshot_with(vec![(
            "cpu",
            SectionData::Cpu(CpuData {
                model: Some("new cpu".to_owned()),
                logical_count: Some(8),
                physical_count: None,
                features: BTreeSet::from(["sse2".to_owned()]),
            }),
        )]);
        let b = snapshot_with(vec![(
            "cpu",
            SectionData::Cpu(CpuData {
                model: Some("new cpu".to_owned()),
                logical_count: None,
                physical_count: None,
                features: BTreeSet::from(["sse2".to_owned()]),
            }),
        )]);
        let diff = diff(&a, &b, &DiffPolicy::compatibility());
        let text = render_text(&diff);
        assert!(
            text.contains("logical_count: 8 -> <absent>"),
            "actual: {text:?}"
        );
    }

    #[test]
    fn json_rendering_produces_the_versioned_envelope() {
        let json = render_json(&sample_diff());
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed["diff_schema"]["name"], DIFF_SCHEMA_NAME);
        assert_eq!(parsed["diff_schema"]["version"], DIFF_SCHEMA_VERSION);
        let changes = parsed["changes"].as_array().expect("array");
        assert!(!changes.is_empty());
        assert_eq!(changes[0]["path"], "cpu.features");
        assert_eq!(changes[0]["kind"], "set_changed");
    }
}
