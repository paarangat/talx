//! Which app is the user dictating into?
//!
//! The frontmost application is captured at hotkey-down (see `register_hotkey`
//! in `lib.rs`), NOT at polish time: by then the user may have clicked the Talx
//! widget or have the settings window open, and we would read our own bundle id.
//!
//! The captured bundle id is mapped to a coarse category, and each category
//! contributes one extra formatting line to the LLM system prompt.

/// The app the user was dictating into when recording started.
/// `allow(dead_code)`: on non-macOS nothing constructs this yet.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct TargetApp {
    pub bundle_id: String,
    /// Localized app name, used in the capture log line.
    pub name: String,
}

/// Returns the frontmost application, or `None` when there is nothing usable to
/// report: no frontmost app, an app without a bundle identifier (this is what a
/// `tauri dev` binary looks like — unbundled), or Talx itself.
///
/// `own_bundle_id` comes from `tauri.conf.json` → `identifier` at runtime, so it
/// can never drift from the real one.
#[cfg(target_os = "macos")]
pub fn frontmost_app(own_bundle_id: &str) -> Option<TargetApp> {
    use cocoa::base::{id, nil};
    use objc::runtime::Class;
    use objc::{msg_send, sel, sel_impl};

    // SAFETY: read-only Objective-C accessors on the NSWorkspace singleton.
    // Every returned pointer is nil-checked before it is messaged, so a missing
    // frontmost app is `None` rather than a crash. Called on the main thread
    // (see `capture_target_app` in lib.rs).
    unsafe {
        // `Class::get` returns Option; the `class!` macro would panic instead.
        let workspace_class = Class::get("NSWorkspace")?;
        let workspace: id = msg_send![workspace_class, sharedWorkspace];
        if workspace == nil {
            return None;
        }

        let app: id = msg_send![workspace, frontmostApplication];
        if app == nil {
            println!("[talx][context] no frontmost application — keeping previous target");
            return None;
        }

        let name_obj: id = msg_send![app, localizedName];
        let bundle_obj: id = msg_send![app, bundleIdentifier];
        let name = nsstring(name_obj).unwrap_or_else(|| "?".to_string());

        let Some(bundle_id) = nsstring(bundle_obj) else {
            println!(
                "[talx][context] frontmost app has no bundle id (name={name}) — keeping previous target"
            );
            return None;
        };

        if bundle_id.eq_ignore_ascii_case(own_bundle_id) {
            println!("[talx][context] frontmost is Talx ({bundle_id}) — keeping previous target");
            return None;
        }

        println!(
            "[talx][context] captured bundle={bundle_id} name={name} category={}",
            classify(&bundle_id).0
        );
        Some(TargetApp { bundle_id, name })
    }
}

/// Copies an `NSString` into an owned `String`. Returns `None` for nil.
///
/// # Safety
/// `s` must be nil or a valid `NSString`.
#[cfg(target_os = "macos")]
unsafe fn nsstring(s: cocoa::base::id) -> Option<String> {
    use cocoa::base::nil;
    use cocoa::foundation::NSString;

    if s == nil {
        return None;
    }
    let ptr = s.UTF8String();
    if ptr.is_null() {
        return None;
    }
    Some(std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned())
}

#[cfg(not(target_os = "macos"))]
pub fn frontmost_app(_own_bundle_id: &str) -> Option<TargetApp> {
    // ponytail: macOS only for now. Windows would use GetForegroundWindow +
    // QueryFullProcessImageName and key the table on the exe name instead.
    None
}

// The extra system-prompt line per category. These are `&'static str` on
// purpose: transcript content can never reach `build_system_prompt`, so the
// prompt-injection hardening in `SYSTEM_PROMPT` cannot be steered by what the
// user dictated.
const TERMINAL: &str = "Output target: a terminal emulator. Emit a bare shell command or plain text — no trailing period, no sentence capitalization, no markdown, no prose formatting.";
const CODE: &str = "Output target: a code editor. Preserve identifiers, paths, and symbols exactly as spoken; markdown and code formatting are allowed; do not force sentence capitalization or add trailing punctuation to non-prose lines.";
const CHAT: &str = "Output target: a chat app. Keep it short and casual, matching how the words were spoken; minimal punctuation; no closing sign-off.";
const EMAIL: &str = "Output target: an email client. Use complete, professional sentences with full punctuation and capitalization.";

/// Maps a bundle identifier to `(category, extra system-prompt line)`.
/// Unknown apps get `("default", None)` — the current, unchanged behaviour.
pub fn classify(bundle_id: &str) -> (&'static str, Option<&'static str>) {
    let id = bundle_id.to_ascii_lowercase();

    // JetBrains ships one bundle id per IDE (com.jetbrains.intellij, .pycharm, …).
    // ponytail: prefix match beats listing a dozen. Deliberately NOT done for
    // Cursor's `com.todesktop.*` — that is a shared Electron namespace.
    if id.starts_with("com.jetbrains.") {
        return ("code", Some(CODE));
    }

    match id.as_str() {
        "com.apple.terminal"
        | "com.googlecode.iterm2"
        | "dev.warp.warp-stable"
        | "com.mitchellh.ghostty"
        | "org.alacritty"
        | "io.alacritty"
        | "net.kovidgoyal.kitty"
        | "com.github.wez.wezterm" => ("terminal", Some(TERMINAL)),

        "com.microsoft.vscode"
        | "com.microsoft.vscodeinsiders"
        | "com.vscodium"
        | "com.todesktop.230313mzl4w4u92" // Cursor
        | "com.apple.dt.xcode"
        | "dev.zed.zed"
        | "com.sublimetext.4"
        | "com.sublimetext.3" => ("code", Some(CODE)),

        "com.tinyspeck.slackmacgap"
        | "com.hnc.discord"
        | "com.apple.mobilesms"
        | "net.whatsapp.whatsapp"
        | "ru.keepcoder.telegram"
        | "com.tdesktop.telegram" => ("chat", Some(CHAT)),

        "com.apple.mail"
        | "com.superhuman.electron"
        | "com.readdle.smartemail-mac"
        | "com.microsoft.outlook" => ("email", Some(EMAIL)),

        // Browsers are the neutral default: a browser tab could be Gmail, a
        // GitHub comment, or a search box. Listed so the log names them.
        "com.google.chrome"
        | "com.apple.safari"
        | "company.thebrowser.browser"
        | "com.brave.browser"
        | "org.mozilla.firefox" => ("browser", None),

        _ => ("default", None),
    }
}

#[cfg(test)]
mod tests {
    use super::classify;

    #[test]
    fn classifies_known_apps_case_insensitively() {
        assert_eq!(classify("com.apple.Terminal").0, "terminal");
        assert_eq!(classify("com.microsoft.VSCode").0, "code");
        assert_eq!(classify("com.jetbrains.intellij").0, "code");
        assert_eq!(classify("com.apple.MobileSMS").0, "chat");
        assert_eq!(classify("com.apple.mail").0, "email");
        assert_eq!(classify("com.apple.Safari"), ("browser", None));
        assert_eq!(classify("md.obsidian"), ("default", None));
    }

    #[test]
    fn known_categories_carry_a_rule() {
        assert!(classify("com.apple.Terminal").1.is_some());
        assert!(classify("com.apple.mail").1.is_some());
    }
}
