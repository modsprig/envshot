use std::ffi::c_int;

use envshot_ffi::{
    ENVSHOT_ABI_VERSION, EnvShotBuffer, envshot_abi_version, envshot_capture_json,
    envshot_diff_json, envshot_fingerprint_json, envshot_free_buffer,
};

const COMPLETE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/snapshots/v0.1/complete-basic.json"
));
const NOISE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/snapshots/v0.1/complete-basic-noise.json"
));

fn read_buffer(buffer: *mut EnvShotBuffer) -> String {
    assert!(!buffer.is_null());

    unsafe {
        let bytes = std::slice::from_raw_parts((*buffer).ptr, (*buffer).len);
        let text = String::from_utf8_lossy(bytes).into_owned();
        envshot_free_buffer(buffer);
        text
    }
}

fn call_fingerprint(snapshot: &str) -> (c_int, String) {
    let mut out: *mut EnvShotBuffer = std::ptr::null_mut();
    let status = unsafe {
        envshot_fingerprint_json(
            snapshot.as_ptr(),
            snapshot.len(),
            std::ptr::null(),
            0,
            std::ptr::from_mut(&mut out),
        )
    };
    let text = if out.is_null() {
        String::new()
    } else {
        read_buffer(out)
    };
    (status, text)
}

#[test]
fn abi_version_is_exported() {
    assert_eq!(envshot_abi_version(), ENVSHOT_ABI_VERSION);
    assert_eq!(ENVSHOT_ABI_VERSION, 1);
}

#[test]
fn capture_with_default_config_succeeds() {
    let mut out: *mut EnvShotBuffer = std::ptr::null_mut();
    let status = unsafe { envshot_capture_json(std::ptr::null(), 0, std::ptr::from_mut(&mut out)) };
    assert_eq!(status, 0);
    let text = read_buffer(out);
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    assert_eq!(parsed["capture"]["privacy"], "standard");
    assert!(parsed["sections"].is_object());
}

#[test]
fn invalid_config_exits_two_without_buffer() {
    let mut out: *mut EnvShotBuffer = std::ptr::null_mut();
    let config = "{ not json";
    let status = unsafe {
        envshot_capture_json(config.as_ptr(), config.len(), std::ptr::from_mut(&mut out))
    };
    assert_eq!(status, 2);
    assert!(out.is_null(), "no buffer for invalid input");
}

#[test]
fn invalid_utf8_config_is_rejected() {
    let mut out: *mut EnvShotBuffer = std::ptr::null_mut();
    let invalid: [u8; 4] = [0xFF, 0xFE, 0x00, 0x01];
    let status = unsafe {
        envshot_capture_json(
            invalid.as_ptr(),
            invalid.len(),
            std::ptr::from_mut(&mut out),
        )
    };
    assert_eq!(status, 2);
    assert!(out.is_null());
}

#[test]
fn fingerprint_matches_the_rust_golden_vector() {
    let (status, text) = call_fingerprint(COMPLETE);
    assert_eq!(status, 0);
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    assert_eq!(
        parsed["fingerprint"],
        "repro-v1:sha256:4119291c8cf461350058f0cedf1de7ff7d5fdb75d5ee6bc0f3045599b32608a7"
    );
    assert_eq!(parsed["projection"]["facts"]["os.family"], "windows");
}

#[test]
fn diff_produces_the_versioned_envelope() {
    let mut out: *mut EnvShotBuffer = std::ptr::null_mut();
    let status = unsafe {
        envshot_diff_json(
            COMPLETE.as_ptr(),
            COMPLETE.len(),
            NOISE.as_ptr(),
            NOISE.len(),
            std::ptr::null(),
            0,
            std::ptr::from_mut(&mut out),
        )
    };
    assert_eq!(status, 0);
    let text = read_buffer(out);
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    assert_eq!(parsed["diff_schema"]["name"], "envshot.diff");
}

#[test]
fn malformed_snapshot_exits_three_with_envelope() {
    let (status, text) = call_fingerprint("{ not json");
    assert_eq!(status, 3);
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("envelope is JSON");
    assert_eq!(parsed["error"]["code"], "snapshot.read.malformed");
}

#[test]
fn repeated_calls_with_frees_are_stable() {
    for _ in 0..64 {
        let (status, text) = call_fingerprint(COMPLETE);
        assert_eq!(status, 0);
        assert!(text.contains("repro-v1"));
    }
}

#[test]
fn free_null_is_a_no_op() {
    unsafe { envshot_free_buffer(std::ptr::null_mut()) };
}
