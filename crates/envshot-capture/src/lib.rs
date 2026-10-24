mod architecture;
mod collector;
mod cpu;
mod distro;
mod gpu;
mod locale;
mod memory;
mod orchestrate;
mod os;
mod registry;
mod registry_reader;
mod virtualization;
mod windows_version;

pub use collector::{Collector, CollectorDescriptor, CollectorError, SectionContribution};
pub use orchestrate::capture_with_registry;
pub use registry::{CaptureError, CollectorRegistry, CollectorRegistryBuilder};
pub use registry_reader::{RegistryReader, default_registry_reader};

use std::sync::Arc;

use envshot_model::{CaptureProfile, Snapshot};

pub const PRODUCER_NAME: &str = "envshot";

#[derive(Clone)]
pub struct CaptureContext {
    pub profile: CaptureProfile,

    pub registry_reader: Arc<dyn RegistryReader>,
}

#[must_use]
pub fn capture(profile: CaptureProfile) -> Snapshot {
    let registry = match profile {
        CaptureProfile::EmptyTest => CollectorRegistry::empty(),
        CaptureProfile::Minimal | CaptureProfile::Standard => CollectorRegistry::builtin(),
    };
    capture_with_registry(&registry, profile)
}
