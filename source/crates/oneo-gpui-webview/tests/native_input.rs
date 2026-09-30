#![cfg(target_os = "macos")]

use std::{process::Command, time::SystemTime};

#[test]
fn native_pointer_delivery_and_cursor_ownership() {
    let unique = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let binary = std::env::temp_dir().join(format!("oneo-webview-input-{unique}"));
    let build = Command::new("clang")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "-fobjc-arc",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-Wno-unused-parameter",
            "tests/native_input.m",
            "-framework",
            "AppKit",
            "-framework",
            "WebKit",
            "-framework",
            "QuartzCore",
            "-o",
        ])
        .arg(&binary)
        .output()
        .expect("compile the detached AppKit test fixture");
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let result = Command::new(&binary).output();
    let _ = std::fs::remove_file(&binary);
    let result = result.expect("run the detached AppKit test fixture");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
