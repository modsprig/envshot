use envshot_model::{Architecture, ArchitectureData, CollectorIssue, IssueCode, SectionStatus};

use crate::CaptureContext;
use crate::collector::{Collector, CollectorDescriptor, CollectorError, SectionContribution};

fn architecture_for(name: &str) -> Architecture {
    match name {
        "x86" => Architecture::X86,
        "x86_64" | "amd64" => Architecture::X64,
        "aarch64" | "arm64" => Architecture::Arm64,
        _ => Architecture::Unknown,
    }
}

pub(crate) fn map_architecture(process_arch: &str, machine_arch: Option<&str>) -> ArchitectureData {
    ArchitectureData {
        process: architecture_for(process_arch),
        os: machine_arch.map_or(Architecture::Unknown, architecture_for),
    }
}

#[cfg(any(windows, test))]
pub(crate) fn architecture_from_wow64(
    process_machine: u16,
    native_machine: u16,
    std_arch: &str,
) -> ArchitectureData {
    let machine = |code: u16| match code {
        332 => Architecture::X86,
        34404 => Architecture::X64,
        43620 => Architecture::Arm64,
        _ => Architecture::Unknown,
    };
    let process = if process_machine == 0 {
        architecture_for(std_arch)
    } else {
        machine(process_machine)
    };
    ArchitectureData {
        process,
        os: machine(native_machine),
    }
}

#[cfg(windows)]
fn collect_architecture() -> ArchitectureData {
    if let Some((process_machine, native_machine)) = wow64_machines() {
        return architecture_from_wow64(process_machine, native_machine, std::env::consts::ARCH);
    }
    map_architecture(
        std::env::consts::ARCH,
        Some(sysinfo::System::cpu_arch().as_str()),
    )
}

#[cfg(windows)]
fn wow64_machines() -> Option<(u16, u16)> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::SystemInformation::{
        IMAGE_FILE_MACHINE, IMAGE_FILE_MACHINE_UNKNOWN,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, IsWow64Process2};

    let mut process_machine = IMAGE_FILE_MACHINE(IMAGE_FILE_MACHINE_UNKNOWN.0);
    let mut native_machine = IMAGE_FILE_MACHINE(IMAGE_FILE_MACHINE_UNKNOWN.0);

    let result = unsafe {
        IsWow64Process2(
            HANDLE(GetCurrentProcess().0),
            std::ptr::from_mut(&mut process_machine),
            Some(std::ptr::from_mut(&mut native_machine)),
        )
    };
    result.ok()?;
    if process_machine == IMAGE_FILE_MACHINE_UNKNOWN {
        Some((0, native_machine.0))
    } else {
        Some((process_machine.0, native_machine.0))
    }
}

#[cfg(not(windows))]
fn collect_architecture() -> ArchitectureData {
    map_architecture(
        std::env::consts::ARCH,
        Some(sysinfo::System::cpu_arch().as_str()),
    )
}

pub(crate) struct ArchitectureCollector;

impl Collector for ArchitectureCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor::new(
            "architecture.host",
            "architecture",
            1,
            envshot_model::SourceKind::LibraryAdapter,
        )
    }

    fn collect(&self, _ctx: &CaptureContext) -> Result<SectionContribution, CollectorError> {
        let data = collect_architecture();

        if data.process == Architecture::Unknown {
            let issue = CollectorIssue::new(
                IssueCode::new("architecture_unavailable").expect("static code"),
            );
            return Ok(SectionContribution::new(
                "architecture",
                SectionStatus::Unavailable,
                None,
                vec![issue],
            ));
        }

        Ok(SectionContribution::new(
            "architecture",
            SectionStatus::Available,
            Some(envshot_model::SectionData::Architecture(data)),
            Vec::new(),
        ))
    }
}

#[cfg(test)]
mod mapping_tests {
    use super::*;

    #[test]
    fn std_arch_names_map_to_the_closed_vocabulary() {
        assert_eq!(architecture_for("x86"), Architecture::X86);
        assert_eq!(architecture_for("x86_64"), Architecture::X64);
        assert_eq!(architecture_for("amd64"), Architecture::X64);
        assert_eq!(architecture_for("aarch64"), Architecture::Arm64);
        assert_eq!(architecture_for("arm64"), Architecture::Arm64);
    }

    #[test]
    fn unknown_names_map_to_unknown() {
        assert_eq!(architecture_for("mips"), Architecture::Unknown);
        assert_eq!(architecture_for(""), Architecture::Unknown);
    }

    #[test]
    fn missing_machine_string_never_guesses_from_the_process_arch() {
        let data = map_architecture("x86_64", None);
        assert_eq!(data.process, Architecture::X64);
        assert_eq!(data.os, Architecture::Unknown);
    }

    #[test]
    fn unmappable_machine_string_leaves_os_unknown() {
        let data = map_architecture("x86_64", Some("sparc"));
        assert_eq!(data.process, Architecture::X64);
        assert_eq!(data.os, Architecture::Unknown);
    }

    #[test]
    fn wow64_style_combination_is_representable() {
        let data = map_architecture("x86", Some("x86_64"));
        assert_eq!(data.process, Architecture::X86);
        assert_eq!(data.os, Architecture::X64);
    }

    #[test]
    fn unmappable_process_arch_degrades_the_contribution() {
        let data = map_architecture("mips", Some("x86_64"));
        assert_eq!(data.process, Architecture::Unknown);
        assert_eq!(data.os, Architecture::X64);
    }

    #[test]
    fn native_x64_process_on_x64_host() {
        let data = architecture_from_wow64(0, 34404, "x86_64");
        assert_eq!(data.process, Architecture::X64);
        assert_eq!(data.os, Architecture::X64);
    }

    #[test]
    fn thirty_two_bit_process_under_wow64_is_representable() {
        let data = architecture_from_wow64(332, 34404, "x86");
        assert_eq!(data.process, Architecture::X86);
        assert_eq!(data.os, Architecture::X64);
    }

    #[test]
    fn x64_process_on_arm64_host_is_representable() {
        let data = architecture_from_wow64(34404, 43620, "x86_64");
        assert_eq!(data.process, Architecture::X64);
        assert_eq!(data.os, Architecture::Arm64);
    }

    #[test]
    fn unknown_machine_codes_map_to_unknown() {
        let data = architecture_from_wow64(0, 9999, "x86_64");
        assert_eq!(data.process, Architecture::X64);
        assert_eq!(data.os, Architecture::Unknown);
    }
}
