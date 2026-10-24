use std::io::Write as _;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use envshot_core::PrivacyPolicy;
use envshot_model::{CaptureProfile, PrivacyProfile, Snapshot};

#[derive(Parser)]
#[command(
    name = "envshot",
    version,
    about = "Capture and compare execution-environment snapshots"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Fingerprint {
        #[arg(value_name = "SNAPSHOT")]
        snapshot: std::path::PathBuf,

        #[arg(long, default_value = "repro-v1")]
        profile: String,

        #[arg(long, default_value = "text")]
        format: String,
    },

    Diff {
        #[arg(value_name = "BEFORE")]
        before: std::path::PathBuf,

        #[arg(value_name = "AFTER")]
        after: std::path::PathBuf,

        #[arg(long, default_value = "text")]
        format: String,

        #[arg(long, default_value = "compatibility")]
        policy: String,
    },

    Capture {
        #[arg(long, default_value = "standard")]
        profile: String,

        #[arg(long, default_value = "standard")]
        privacy: String,

        #[arg(long, default_value = "json")]
        format: String,

        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Capture {
            profile,
            privacy,
            format,
            output,
        } => cmd_capture(&profile, &privacy, &format, output.as_deref()),
        Command::Diff {
            before,
            after,
            format,
            policy,
        } => cmd_diff(&before, &after, &format, &policy),
        Command::Fingerprint {
            snapshot,
            profile,
            format,
        } => cmd_fingerprint(&snapshot, &profile, &format),
    }
}

fn cmd_fingerprint(path: &std::path::Path, profile: &str, format: &str) -> ExitCode {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            return input_error(&format!("could not read {}: {error}", path.display()));
        }
    };
    let snapshot = match envshot_model::read_snapshot(&content) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return input_error(&format!("{}: {error}", path.display()));
        }
    };
    let profile = match profile {
        "repro-v1" => envshot_core::FingerprintProfile::ReproV1,
        _ => return usage_error("unknown --profile (expected repro-v1)"),
    };
    let fingerprint = match envshot_core::fingerprint(&snapshot, &profile) {
        Ok(fingerprint) => fingerprint,
        Err(error) => return input_error(&error.to_string()),
    };

    let rendered = match format {
        "text" => fingerprint.value,
        "json" => {
            let projection: serde_json::Value = match serde_json::from_str(&fingerprint.projection)
            {
                Ok(projection) => projection,
                Err(error) => return internal_error(&format!("projection rendering: {error}")),
            };
            serde_json::json!({
                "fingerprint_schema": {"name": "envshot.fingerprint", "version": 1},
                "fingerprint": fingerprint.value,
                "profile": profile.id(),
                "projection": projection,
            })
            .to_string()
        }
        _ => return usage_error("unknown --format (expected text|json)"),
    };

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    match (writeln!(handle, "{rendered}"), handle.flush()) {
        (Ok(()), Ok(())) => ExitCode::SUCCESS,
        _ => internal_error("could not write to stdout"),
    }
}

fn cmd_diff(
    before: &std::path::Path,
    after: &std::path::Path,
    format: &str,
    policy: &str,
) -> ExitCode {
    let read = |path: &std::path::Path| -> Result<Snapshot, ExitCode> {
        let content = std::fs::read_to_string(path)
            .map_err(|error| input_error(&format!("could not read {}: {error}", path.display())))?;
        envshot_model::read_snapshot(&content)
            .map_err(|error| input_error(&format!("{}: {}", path.display(), error)))
    };

    let before = match read(before) {
        Ok(snapshot) => snapshot,
        Err(code) => return code,
    };
    let after = match read(after) {
        Ok(snapshot) => snapshot,
        Err(code) => return code,
    };

    let policy = match policy {
        "compatibility" => envshot_core::DiffPolicy::compatibility(),
        "diagnostic" => envshot_core::DiffPolicy::diagnostic(),
        _ => return usage_error("unknown --policy (expected compatibility|diagnostic)"),
    };
    let diff = envshot_core::diff(&before, &after, &policy);

    let rendered = match format {
        "text" => envshot_core::render_text(&diff),
        "json" => envshot_core::render_json(&diff),
        _ => return usage_error("unknown --format (expected text|json)"),
    };

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    match (writeln!(handle, "{rendered}"), handle.flush()) {
        (Ok(()), Ok(())) => ExitCode::SUCCESS,
        _ => internal_error("could not write to stdout"),
    }
}

fn input_error(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(3)
}

const PRIVACY_OFF_WARNING: &str = "warning: --privacy off disables identifier minimization; the output is for local debugging only";

fn parse_profile(value: &str) -> Option<CaptureProfile> {
    match value {
        "minimal" => Some(CaptureProfile::Minimal),
        "standard" => Some(CaptureProfile::Standard),
        "empty-test" => Some(CaptureProfile::EmptyTest),
        _ => None,
    }
}

fn parse_privacy(value: &str) -> Option<PrivacyPolicy> {
    PrivacyProfile::deserialize_from_str(value).map(PrivacyPolicy::from_profile)
}

trait DeserializeFromStr {
    fn deserialize_from_str(value: &str) -> Option<PrivacyProfile>;
}

impl DeserializeFromStr for PrivacyProfile {
    fn deserialize_from_str(value: &str) -> Option<PrivacyProfile> {
        match value {
            "strict" => Some(PrivacyProfile::Strict),
            "standard" => Some(PrivacyProfile::Standard),
            "off" => Some(PrivacyProfile::Off),
            _ => None,
        }
    }
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(2)
}

fn internal_error(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(5)
}

fn cmd_capture(
    profile: &str,
    privacy: &str,
    format: &str,
    output: Option<&std::path::Path>,
) -> ExitCode {
    let Some(profile) = parse_profile(profile) else {
        return usage_error("unknown --profile (expected minimal|standard|empty-test)");
    };
    let Some(policy) = parse_privacy(privacy) else {
        return usage_error("unknown --privacy (expected strict|standard|off)");
    };
    let pretty = match format {
        "json" => false,
        "pretty-json" => true,
        _ => return usage_error("unknown --format (expected json|pretty-json)"),
    };

    if privacy == "off" {
        eprintln!("{PRIVACY_OFF_WARNING}");
    }

    let snapshot = envshot::capture_with_privacy(profile, &policy);
    let rendered = if pretty {
        serde_json::to_string_pretty(&snapshot)
    } else {
        serde_json::to_string(&snapshot)
    };
    let rendered = match rendered {
        Ok(rendered) => rendered,
        Err(error) => return internal_error(&format!("serialization failed: {error}")),
    };

    if let Some(path) = output {
        return match std::fs::write(path, rendered + "\n") {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => internal_error(&format!("could not write output: {error}")),
        };
    }

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    match (writeln!(handle, "{rendered}"), handle.flush()) {
        (Ok(()), Ok(())) => ExitCode::SUCCESS,
        _ => internal_error("could not write to stdout"),
    }
}

#[cfg(test)]
mod exit_code_tests {
    use super::*;

    #[test]
    fn internal_errors_exit_with_code_five() {
        assert_eq!(internal_error("boom"), ExitCode::from(5));
    }

    #[test]
    fn profile_and_privacy_parsers_cover_the_contract() {
        assert!(parse_profile("minimal").is_some());
        assert!(parse_profile("standard").is_some());
        assert!(parse_profile("empty-test").is_some());
        assert!(parse_profile("bogus").is_none());
        assert!(parse_privacy("strict").is_some());
        assert!(parse_privacy("standard").is_some());
        assert!(parse_privacy("off").is_some());
        assert!(parse_privacy("bogus").is_none());
    }
}
