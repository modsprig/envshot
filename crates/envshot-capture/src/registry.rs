use crate::architecture::ArchitectureCollector;
use crate::collector::{Collector, CollectorDescriptor};
use crate::cpu::CpuCollector;
use crate::distro::DistroCollector;
use crate::gpu::GpuCollector;
use crate::locale::LocaleCollector;
use crate::memory::MemoryCollector;
use crate::os::OsCollector;
use crate::virtualization::VirtualizationCollector;
use crate::windows_version::WindowsVersionCollector;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CaptureError {
    #[error("duplicate collector id: {id}")]
    DuplicateCollector { id: String },

    #[error("duplicate section id: {id}")]
    DuplicateSection { id: String },

    #[error("invalid collector descriptor: {reason}")]
    InvalidDescriptor { reason: String },
}

impl CaptureError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::DuplicateCollector { .. } => "capture.duplicate_collector",
            Self::DuplicateSection { .. } => "capture.duplicate_section",
            Self::InvalidDescriptor { .. } => "capture.invalid_descriptor",
        }
    }
}

#[derive(Default)]
pub struct CollectorRegistry {
    collectors: Vec<Box<dyn Collector>>,
}

impl std::fmt::Debug for CollectorRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollectorRegistry")
            .field("collectors", &self.collectors.len())
            .finish()
    }
}

impl CollectorRegistry {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn builtin() -> Self {
        let mut builder = Self::builder();
        builder = builder
            .register(OsCollector)
            .expect("built-in descriptors are unique and non-empty");
        builder = builder
            .register(ArchitectureCollector)
            .expect("built-in descriptors are unique and non-empty");
        builder = builder
            .register(CpuCollector)
            .expect("built-in descriptors are unique and non-empty");
        builder = builder
            .register(MemoryCollector)
            .expect("built-in descriptors are unique and non-empty");
        builder = builder
            .register(LocaleCollector)
            .expect("built-in descriptors are unique and non-empty");
        builder = builder
            .register(VirtualizationCollector)
            .expect("built-in descriptors are unique and non-empty");
        builder = builder
            .register(WindowsVersionCollector)
            .expect("built-in descriptors are unique and non-empty");
        builder = builder
            .register(GpuCollector)
            .expect("built-in descriptors are unique and non-empty");
        builder = builder
            .register(DistroCollector)
            .expect("built-in descriptors are unique and non-empty");
        builder.build()
    }

    #[must_use]
    pub fn builder() -> CollectorRegistryBuilder {
        CollectorRegistryBuilder {
            collectors: Vec::new(),
        }
    }

    pub(crate) fn collectors(&self) -> &[Box<dyn Collector>] {
        &self.collectors
    }
}

#[derive(Default)]
pub struct CollectorRegistryBuilder {
    collectors: Vec<Box<dyn Collector>>,
}

impl std::fmt::Debug for CollectorRegistryBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollectorRegistryBuilder")
            .field("collectors", &self.collectors.len())
            .finish()
    }
}

impl CollectorRegistryBuilder {
    pub fn register(mut self, collector: impl Collector + 'static) -> Result<Self, CaptureError> {
        let descriptor: CollectorDescriptor = collector.descriptor();
        if descriptor.id.is_empty() {
            return Err(CaptureError::InvalidDescriptor {
                reason: "collector id must not be empty".to_owned(),
            });
        }
        if descriptor.section_id.is_empty() {
            return Err(CaptureError::InvalidDescriptor {
                reason: "section id must not be empty".to_owned(),
            });
        }
        if self
            .collectors
            .iter()
            .any(|existing| existing.descriptor().id == descriptor.id)
        {
            return Err(CaptureError::DuplicateCollector { id: descriptor.id });
        }
        if self
            .collectors
            .iter()
            .any(|existing| existing.descriptor().section_id == descriptor.section_id)
        {
            return Err(CaptureError::DuplicateSection {
                id: descriptor.section_id,
            });
        }
        self.collectors.push(Box::new(collector));
        Ok(self)
    }

    #[must_use]
    pub fn build(self) -> CollectorRegistry {
        CollectorRegistry {
            collectors: self.collectors,
        }
    }
}
