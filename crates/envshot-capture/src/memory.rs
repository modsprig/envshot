use envshot_model::{CollectorIssue, IssueCode, MemoryData, SectionStatus};

use crate::CaptureContext;
use crate::collector::{Collector, CollectorDescriptor, CollectorError, SectionContribution};

pub(crate) fn memory_contribution(total_bytes: u64) -> SectionContribution {
    if total_bytes == 0 {
        let issue = CollectorIssue::new(IssueCode::new("memory_unavailable").expect("static code"));
        return SectionContribution::new("memory", SectionStatus::Unavailable, None, vec![issue]);
    }
    SectionContribution::new(
        "memory",
        SectionStatus::Available,
        Some(envshot_model::SectionData::Memory(MemoryData {
            installed_bytes: total_bytes,
        })),
        Vec::new(),
    )
}

pub(crate) struct MemoryCollector;

impl Collector for MemoryCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor::new(
            "memory.host",
            "memory",
            1,
            envshot_model::SourceKind::LibraryAdapter,
        )
    }

    fn collect(&self, _ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
        let mut system = sysinfo::System::new();
        system.refresh_memory();
        Ok(memory_contribution(system.total_memory()))
    }
}

#[cfg(test)]
mod mapping_tests {
    use super::*;

    #[test]
    fn positive_memory_is_available() {
        let contribution = memory_contribution(17_179_869_184);
        assert_eq!(contribution.status, SectionStatus::Available);
        let Some(envshot_model::SectionData::Memory(memory)) = contribution.data else {
            panic!("memory contribution must carry MemoryData");
        };
        assert_eq!(memory.installed_bytes, 17_179_869_184);
    }

    #[test]
    fn zero_memory_degrades_to_unavailable() {
        let contribution = memory_contribution(0);
        assert_eq!(contribution.status, SectionStatus::Unavailable);
        assert_eq!(contribution.issues[0].code.as_str(), "memory_unavailable");
    }
}
