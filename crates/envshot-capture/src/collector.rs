use envshot_model::{CollectorIssue, SectionData, SectionStatus, SourceKind};

use crate::CaptureContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorDescriptor {
    pub id: String,

    pub section_id: String,

    pub revision: u32,

    pub source_kind: SourceKind,
}

impl CollectorDescriptor {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        section_id: impl Into<String>,
        revision: u32,
        source_kind: SourceKind,
    ) -> Self {
        Self {
            id: id.into(),
            section_id: section_id.into(),
            revision,
            source_kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionContribution {
    pub section_id: String,

    pub status: SectionStatus,

    pub data: Option<SectionData>,

    pub issues: Vec<CollectorIssue>,
}

impl SectionContribution {
    #[must_use]
    pub fn new(
        section_id: impl Into<String>,
        status: SectionStatus,
        data: Option<SectionData>,
        issues: Vec<CollectorIssue>,
    ) -> Self {
        Self {
            section_id: section_id.into(),
            status,
            data,
            issues,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("collector internal error: {0}")]
pub struct CollectorError(pub String);

impl CollectorError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

pub trait Collector {
    fn descriptor(&self) -> CollectorDescriptor;

    fn collect(&self, ctx: &CaptureContext) -> Result<SectionContribution, CollectorError>;
}
