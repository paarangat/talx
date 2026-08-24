//! Accessibility permission checks.
//!
//! macOS silently no-ops synthetic key events (the Cmd+V we send to paste)
//! unless the app is listed under Privacy & Security > Accessibility.

/// Deep link to the Accessibility pane of System Settings.
#[cfg(target_os = "macos")]
const ACCESSIBILITY_PANE: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    // Returns a CoreFoundation `Boolean`, i.e. `unsigned char` — declared as
    // u8 because returning `bool` from FFI is UB for any value but 0 or 1.
    fn AXIsProcessTrusted() -> u8;
}

#[cfg(target_os = "macos")]
pub fn is_accessibility_trusted() -> bool {
    // SAFETY: no arguments, no pointers; reads a process-global flag.
    unsafe { AXIsProcessTrusted() != 0 }
}

/// Other platforms don't gate synthetic input this way.
#[cfg(not(target_os = "macos"))]
pub fn is_accessibility_trusted() -> bool {
    true
}

#[cfg(target_os = "macos")]
pub fn open_accessibility_settings() -> Result<(), String> {
    std::process::Command::new("open")
        .arg(ACCESSIBILITY_PANE)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to open System Settings: {e}"))
}

#[cfg(not(target_os = "macos"))]
pub fn open_accessibility_settings() -> Result<(), String> {
    Err("Accessibility permission is only required on macOS".to_string())
}
