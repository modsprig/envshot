use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use envshot_model::{
    CaptureMetadata, CaptureProfile, CollectorIssue, IssueCode, PrivacyProfile, ProducerInfo,
    Provenance, SchemaIdentifier, Section, SectionData, SectionStatus, Snapshot,
};

use crate::collector::{CollectorDescriptor, SectionContribution};
use crate::registry::CollectorRegistry;

#[must_use]
pub fn capture_with_registry(registry: &CollectorRegistry, profile: CaptureProfile) -> Snapshot {
    let ctx = crate::CaptureContext {
        profile,
        registry_reader: crate::default_registry_reader(),
    };
    let mut sections = BTreeMap::new();

    for collector in registry.collectors() {
        let descriptor = collector.descriptor();
        let section = match catch_unwind(AssertUnwindSafe(|| collector.collect(&ctx))) {
            Ok(Ok(contribution)) => section_from_contribution(&descriptor, contribution),
            Ok(Err(_)) => degraded_section(&descriptor, "collector_failed"),
            Err(_) => degraded_section(&descriptor, "collector_panicked"),
        };
        sections.insert(descriptor.section_id.clone(), section);
    }

    Snapshot {
        schema: SchemaIdentifier::current(),
        producer: ProducerInfo {
            name: crate::PRODUCER_NAME.to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        },
        capture: CaptureMetadata {
            profile,
            privacy: PrivacyProfile::Standard,
        },
        sections,
    }
}

fn section_from_contribution(
    descriptor: &CollectorDescriptor,
    contribution: SectionContribution,
) -> Section<SectionData> {
    match Section::new(contribution.status, contribution.data, contribution.issues) {
        Ok(section) => section.with_provenance(vec![provenance_for(descriptor)]),
        Err(_) => degraded_section(descriptor, "collector_invalid_contribution"),
    }
}

fn degraded_section(descriptor: &CollectorDescriptor, code: &str) -> Section<SectionData> {
    let issue =
        CollectorIssue::new(IssueCode::new(code).expect("static issue codes are non-empty"));
    Section::new(SectionStatus::Failed, None, vec![issue])
        .expect("failed with one issue is a valid section")
        .with_provenance(vec![provenance_for(descriptor)])
}

fn provenance_for(descriptor: &CollectorDescriptor) -> Provenance {
    Provenance::new(
        descriptor.id.clone(),
        descriptor.revision,
        descriptor.source_kind,
    )
    .expect("registry validated non-empty collector ids")
}
