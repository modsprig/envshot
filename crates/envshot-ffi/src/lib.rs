use std::ffi::c_int;

use serde_json::Value;

pub const ENVSHOT_ABI_VERSION: u32 = 1;

#[repr(C)]
pub struct EnvShotBuffer {
    pub ptr: *mut u8,

    pub len: usize,

    pub capacity: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn envshot_abi_version() -> u32 {
    ENVSHOT_ABI_VERSION
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn envshot_free_buffer(buffer: *mut EnvShotBuffer) {
    if buffer.is_null() {
        return;
    }

    unsafe {
        let boxed = Box::from_raw(buffer);
        if !boxed.ptr.is_null() && boxed.capacity > 0 {
            drop(Vec::from_raw_parts(boxed.ptr, boxed.len, boxed.capacity));
        }
    }
}

fn allocate(json: &str) -> *mut EnvShotBuffer {
    let mut bytes = json.as_bytes().to_vec();
    let ptr = bytes.as_mut_ptr();
    let len = bytes.len();
    let capacity = bytes.capacity();
    std::mem::forget(bytes);
    Box::into_raw(Box::new(EnvShotBuffer { ptr, len, capacity }))
}

fn input_string(ptr: *const u8, len: usize) -> Result<Option<String>, ()> {
    if ptr.is_null() {
        return Ok(None);
    }

    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(Some(text.to_owned())),
        Err(_) => Err(()),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn envshot_capture_json(
    config_ptr: *const u8,
    config_len: usize,
    out: *mut *mut EnvShotBuffer,
) -> c_int {
    let result = std::panic::catch_unwind(|| {
        let config = match input_string(config_ptr, config_len) {
            Ok(Some(config)) => config,
            Ok(None) => String::new(),
            Err(()) => return (2, None),
        };
        let config: Value = if config.trim().is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            match serde_json::from_str::<Value>(&config) {
                Ok(value) => value,
                Err(_) => return (2, None),
            }
        };

        let profile = match config.get("profile").and_then(Value::as_str) {
            None | Some("standard") => envshot_model::CaptureProfile::Standard,
            Some("minimal") => envshot_model::CaptureProfile::Minimal,
            Some("empty_test") => envshot_model::CaptureProfile::EmptyTest,
            Some(_) => return (2, None),
        };
        let policy = match config.get("privacy").and_then(Value::as_str) {
            None | Some("standard") => envshot_core::PrivacyPolicy::standard(),
            Some("strict") => envshot_core::PrivacyPolicy::strict(),
            Some("off") => envshot_core::PrivacyPolicy::off(),
            Some(_) => return (2, None),
        };

        let snapshot = envshot::capture_with_privacy(profile, &policy);
        match serde_json::to_string(&snapshot) {
            Ok(json) => (0, Some(json)),
            Err(_) => (
                1,
                Some(error_envelope_text(
                    "internal.error",
                    "serialization failed",
                )),
            ),
        }
    })
    .unwrap_or((
        1,
        Some(error_envelope_text("internal.panic", "capture panicked")),
    ));

    write_out(result, out)
}

fn error_envelope_text(code: &str, message: &str) -> String {
    serde_json::json!({ "error": { "code": code, "message": message } }).to_string()
}

fn write_out((status, payload): (c_int, Option<String>), out: *mut *mut EnvShotBuffer) -> c_int {
    if let Some(json) = payload.filter(|_| !out.is_null()) {
        unsafe { *out = allocate(&json) };
    }
    status
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn envshot_diff_json(
    a_ptr: *const u8,
    a_len: usize,
    b_ptr: *const u8,
    b_len: usize,
    policy_ptr: *const u8,
    policy_len: usize,
    out: *mut *mut EnvShotBuffer,
) -> c_int {
    let result = std::panic::catch_unwind(|| {
        let (Ok(Some(a_text)), Ok(Some(b_text))) = (
            input_string(a_ptr, a_len),
            input_string(b_ptr, b_len),
        ) else {
            return (2, None);
        };
        let policy = match input_string(policy_ptr, policy_len) {
            Ok(Some(text)) if !text.trim().is_empty() => match text.trim() {
                "\"compatibility\"" | "compatibility" => envshot_core::DiffPolicy::compatibility(),
                "\"diagnostic\"" | "diagnostic" => envshot_core::DiffPolicy::diagnostic(),
                _ => return (2, None),
            },
            _ => envshot_core::DiffPolicy::compatibility(),
        };

        let a = match envshot::read_snapshot(&a_text) {
            Ok(snapshot) => snapshot,
            Err(error) => return (3, Some(error_envelope_text(error.code(), &error.to_string()))),
        };
        let b = match envshot::read_snapshot(&b_text) {
            Ok(snapshot) => snapshot,
            Err(error) => return (3, Some(error_envelope_text(error.code(), &error.to_string()))),
        };

        let diff = envshot::diff(&a, &b, &policy);
        let document = serde_json::json!({
            "diff_schema": {"name": envshot_core::DIFF_SCHEMA_NAME, "version": envshot_core::DIFF_SCHEMA_VERSION},
            "changes": diff.changes(),
        });
        (0, Some(document.to_string()))
    })
    .unwrap_or((1, Some(error_envelope_text("internal.panic", "diff panicked"))));

    write_out(result, out)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn envshot_fingerprint_json(
    snapshot_ptr: *const u8,
    snapshot_len: usize,
    profile_ptr: *const u8,
    profile_len: usize,
    out: *mut *mut EnvShotBuffer,
) -> c_int {
    let result = std::panic::catch_unwind(|| {
        let Ok(Some(text)) = input_string(snapshot_ptr, snapshot_len) else {
            return (2, None);
        };
        let profile = match input_string(profile_ptr, profile_len) {
            Ok(Some(text)) if !text.trim().is_empty() => match text.trim() {
                "\"repro-v1\"" | "repro-v1" => envshot_core::FingerprintProfile::ReproV1,
                _ => return (2, None),
            },
            _ => envshot_core::FingerprintProfile::ReproV1,
        };

        let snapshot = match envshot::read_snapshot(&text) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return (
                    3,
                    Some(error_envelope_text(error.code(), &error.to_string())),
                );
            }
        };
        match envshot::fingerprint(&snapshot, &profile) {
            Ok(fingerprint) => {
                let projection: Value =
                    serde_json::from_str(&fingerprint.projection).unwrap_or(Value::Null);
                let document = serde_json::json!({
                    "fingerprint": fingerprint.value,
                    "projection": projection,
                });
                (0, Some(document.to_string()))
            }
            Err(error) => (
                3,
                Some(error_envelope_text(
                    "fingerprint.unsupported_schema_major",
                    &error.to_string(),
                )),
            ),
        }
    })
    .unwrap_or((
        1,
        Some(error_envelope_text(
            "internal.panic",
            "fingerprint panicked",
        )),
    ));

    write_out(result, out)
}
