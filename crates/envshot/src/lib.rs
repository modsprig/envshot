use envshot_capture::CollectorRegistry;
use envshot_core::PrivacyPolicy;
use envshot_model::{CaptureProfile, Snapshot};

#[must_use]
pub fn capture(profile: CaptureProfile) -> Snapshot {
    capture_with_privacy(profile, &PrivacyPolicy::standard())
}

#[must_use]
pub fn capture_with_privacy(profile: CaptureProfile, policy: &PrivacyPolicy) -> Snapshot {
    let raw = envshot_capture::capture(profile);
    apply(&raw, *policy)
}

#[must_use]
pub fn capture_with_registry(
    registry: &CollectorRegistry,
    profile: CaptureProfile,
    policy: &PrivacyPolicy,
) -> Snapshot {
    let raw = envshot_capture::capture_with_registry(registry, profile);
    apply(&raw, *policy)
}

fn apply(raw: &Snapshot, policy: PrivacyPolicy) -> Snapshot {
    envshot_core::apply_privacy(raw, &policy)
}

pub fn read_snapshot(input: &str) -> Result<Snapshot, envshot_model::SnapshotReadError> {
    envshot_model::read_snapshot(input)
}

#[must_use]
pub fn diff(
    a: &Snapshot,
    b: &Snapshot,
    policy: &envshot_core::DiffPolicy,
) -> envshot_core::SnapshotDiff {
    envshot_core::diff(a, b, policy)
}

pub fn fingerprint(
    snapshot: &Snapshot,
    profile: &envshot_core::FingerprintProfile,
) -> Result<envshot_core::Fingerprint, envshot_core::FingerprintError> {
    envshot_core::fingerprint(snapshot, profile)
}
