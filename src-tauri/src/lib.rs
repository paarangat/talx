use rusqlite::Connection;
use std::sync::Mutex;
use tauri::Emitter;
use tauri::Manager;

mod asr;
mod audio;
mod context;
mod db;
mod dictionary;
mod export;
mod llm;
mod permissions;

use tokio::sync::mpsc;

struct TrayState {
    status_item: tauri::menu::MenuItem<tauri::Wry>,
    words_item: tauri::menu::MenuItem<tauri::Wry>,
}

#[allow(dead_code)]
enum RecordingStatus {
    Idle,
    Recording,
    Transcribing,
    Polishing,
    Result,
}

/// Wrapper to allow AudioCapture (which contains cpal::Stream, a !Send type)
/// to be stored in a Send+Sync state container. Safety: AudioCapture is only
/// accessed while the Mutex is held, ensuring single-threaded access.
struct SendableCapture(Option<audio::AudioCapture>);

// SAFETY: AudioCapture is only accessed behind a Mutex, guaranteeing
// exclusive access. The cpal::Stream inside is never moved across threads
// while active — it is created and dropped on the same async runtime thread.
unsafe impl Send for SendableCapture {}
unsafe impl Sync for SendableCapture {}

struct AppState {
    recording_status: RecordingStatus,
    audio_capture: SendableCapture,
    soniox_session: Option<asr::soniox::SonioxSession>,
    asr_provider: asr::AsrProvider,
    asr_model: String,
    audio_buffer: Option<std::sync::Arc<Mutex<Vec<i16>>>>,
    groq_api_key: String,
    soniox_api_key: String,
    llm_provider: llm::LlmProvider,
    llm_model: String,
    llm_groq_api_key: String,
    llm_openai_api_key: String,
    current_hotkey: String,
    dictionary: Vec<String>,
    db_conn: Connection,
    key_down_time: Option<std::time::Instant>,
    hold_mode: bool,
    /// App the user was dictating into, captured at hotkey-down. Sticky: a
    /// capture that lands on Talx itself leaves the previous value in place.
    target_app: Option<context::TargetApp>,
    context_aware: bool,
    /// Recognition language as a bare ISO-639-1 code; empty means auto-detect.
    language: String,
    /// Generation of the current dictation job. `cancel_dictation` bumps it;
    /// the task `stop_recording` spawned compares against it before emitting,
    /// so a transcription that lands after the user cancelled is dropped
    /// instead of pasted. Same trick as `polishGenRef` in `App.tsx`.
    job_gen: u64,
}

/// Settings-table key for the chosen input device name. Empty/absent = system default.
const INPUT_DEVICE_KEY: &str = "talx:input-device";

/// SQLite `settings` key for the context-aware polish toggle. Default is ON;
/// only an explicit "false" turns it off.
const CONTEXT_AWARE_KEY: &str = "talx:context-aware";

/// SQLite `settings` key for the recognition language. Default (absent) = auto-detect.
const LANGUAGE_KEY: &str = "talx:language";

/// Keep only a bare ISO-639-1 code; everything else — including the legacy
/// "hi+en" this setting used to store — collapses to auto-detect. Whisper
/// rejects codes it doesn't know, so a stray value must never reach the form.
fn normalize_language(value: &str) -> String {
    if value.len() == 2 && value.bytes().all(|b| b.is_ascii_lowercase()) {
        value.to_string()
    } else {
        String::new()
    }
}

fn default_hotkey() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "ctrl+shift+space"
    }

    #[cfg(not(target_os = "windows"))]
    {
        "alt+space"
    }
}

/// Settings key for the (opt-in) clipboard restore. See `RESTORE_DELAY_MS`.
const RESTORE_CLIPBOARD_KEY: &str = "talx:restore-clipboard";

/// Machine-readable paste failures. The frontend matches these exactly and owns
/// all user-facing copy — do not append prose to them.
const ERR_ACCESSIBILITY: &str = "ACCESSIBILITY_DENIED";
const ERR_CLIPBOARD_ONLY: &str = "CLIPBOARD_ONLY";

// How long to leave the transcript on the clipboard before putting the user's
// previous contents back. The target app reads the clipboard asynchronously
// after Cmd+V, so restoring too early cancels the paste.
//
// Guaranteed regardless of this number: the restore only runs when
// Accessibility was still trusted *after* the keystroke (so the paste was not
// dropped by a mid-session revocation) AND the clipboard still holds exactly
// the text we wrote (see `should_restore`, which also stops us clobbering
// something the user copied during the delay).
//
// NOT guaranteed: a keystroke the target app itself ignored — no focused text
// field, or an app that blocks synthetic paste. enigo reports Ok, Accessibility
// is still trusted, our text is still on the clipboard, and the restore wipes
// it. There is no cheap signal for that case, which is why this setting stays
// opt-in and off by default.
//
// ponytail: 800ms itself is still a guess, not a measurement — nobody has
// timed how long a slow target app (Slack, Mail, an Electron editor) takes to
// read the pasteboard after the keystroke. Measure on real hardware before
// trusting it; not a setting, because a number the user cannot measure either
// is not a knob worth shipping.
const RESTORE_DELAY_MS: u64 = 800;

/// Whether it is safe to put the user's previous clipboard back.
///
/// Only when the clipboard still holds exactly the text we wrote. Anything
/// else means either the user copied something during `RESTORE_DELAY_MS`
/// (theirs wins) or the clipboard is not text at all (`None`) — and if our
/// text is gone we have nothing to hand back to, so restoring would only
/// clobber someone else's copy.
fn should_restore(current: Option<&str>, ours: &str) -> bool {
    !ours.is_empty() && current == Some(ours)
}

/// Copies `text` without pasting. Offered by the result card when auto-paste
/// is off, where the transcript would otherwise only exist in history.
///
/// Goes through the same main-thread hop as `paste_to_focused_app` because
/// arboard's clipboard is not safe to touch from a worker thread on macOS.
#[tauri::command]
async fn copy_to_clipboard(app: tauri::AppHandle, text: String) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    app.run_on_main_thread(move || {
        let result = arboard::Clipboard::new()
            .map_err(|e| format!("Clipboard error: {e}"))
            .and_then(|mut c| {
                c.set_text(&text)
                    .map_err(|e| format!("Failed to write clipboard: {e}"))
            });
        let _ = tx.send(result);
    })
    .map_err(|e| e.to_string())?;

    rx.await.map_err(|e| e.to_string())?
}

/// Puts `text` on the clipboard and sends Cmd+V (Ctrl+V off macOS), falling
/// back to typing the text directly. Returns the previous clipboard text when
/// the caller should restore it — only on a Cmd+V that enigo sent and that
/// Accessibility did not drop. Every other path leaves the text on the
/// clipboard for the user and says so via `ERR_ACCESSIBILITY` /
/// `ERR_CLIPBOARD_ONLY`. See `RESTORE_DELAY_MS` for what this does not cover.
#[tauri::command]
async fn paste_to_focused_app(app: tauri::AppHandle, text: String) -> Result<(), String> {
    let restore_enabled = {
        let state = app.state::<Mutex<AppState>>();
        let app_state = state.lock().map_err(|e| e.to_string())?;
        let stored = db::get_setting(&app_state.db_conn, RESTORE_CLIPBOARD_KEY)
            .ok()
            .flatten();
        stored.as_deref() == Some("true")
    };

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<Option<String>, String>>();

    // Kept out of the closure so the restore can check the clipboard still
    // holds what we wrote before overwriting it. Must stay byte-identical to
    // what `set_text` below writes, or the restore silently stops running.
    let ours = text.clone();

    app.run_on_main_thread(move || {
        let result = (|| -> Result<Option<String>, String> {
            use arboard::Clipboard;
            use enigo::{Enigo, Key, Keyboard, Settings};

            let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard error: {e}"))?;

            // `get_text` errors on an empty or non-text clipboard — that is the
            // common case, not a failure, and `None` means "skip the restore"
            // so we never clobber an image or file the user had copied.
            let previous = if restore_enabled {
                clipboard.get_text().ok().filter(|t| !t.is_empty())
            } else {
                None
            };

            clipboard
                .set_text(&text)
                .map_err(|e| format!("Failed to write clipboard: {e}"))?;

            // Checked after the clipboard write so the transcript survives even
            // though we can't paste it. Returning here drops `previous`, so no
            // restore is scheduled — the user needs our text on the clipboard.
            if !permissions::is_accessibility_trusted() {
                return Err(ERR_ACCESSIBILITY.to_string());
            }

            // Small delay for clipboard to settle (main thread is fine — only 50ms, no UI work)
            std::thread::sleep(std::time::Duration::from_millis(50));

            let mut enigo =
                Enigo::new(&Settings::default()).map_err(|_| ERR_CLIPBOARD_ONLY.to_string())?;

            #[cfg(target_os = "macos")]
            let modifier = Key::Meta;
            #[cfg(not(target_os = "macos"))]
            let modifier = Key::Control;

            let pasted = (|| -> Result<(), enigo::InputError> {
                enigo.key(modifier, enigo::Direction::Press)?;
                let click = enigo.key(Key::Unicode('v'), enigo::Direction::Click);
                // Release the modifier even if the click failed, or it stays
                // stuck down for every later keystroke.
                let release = enigo.key(modifier, enigo::Direction::Release);
                click.and(release)
            })();

            // enigo reports Ok for a keystroke macOS silently dropped, which is
            // exactly what happens when Accessibility is revoked mid-session —
            // so `pasted` alone cannot tell us the paste landed. Re-checking
            // the permission is the cheap signal that it did not.
            //
            // Checked before the fallback too, not just the Cmd+V: `enigo.text`
            // posts CGEvents through the same AX-gated path, so it no-ops just
            // as silently and would otherwise return Ok with nothing typed.
            //
            // Err, not Ok(None), even though the keystroke may have landed in
            // the few-millisecond window before the revocation. Both skip the
            // restore, but only Err tells the user the transcript is sitting on
            // the clipboard waiting for a manual ⌘V. A silent Ok would leave
            // them believing it pasted, and the next dictation would overwrite
            // the clipboard for real. The cost is a possible double-paste —
            // visible and undoable, unlike a lost transcript.
            if !permissions::is_accessibility_trusted() {
                return Err(ERR_ACCESSIBILITY.to_string());
            }

            if pasted.is_ok() {
                return Ok(previous);
            }

            // Fallback: type the text directly. Slower, but works where
            // synthetic paste does not. The text stays on the clipboard, so
            // never restore from here on.
            enigo
                .text(&text)
                .map(|()| None)
                .map_err(|_| ERR_CLIPBOARD_ONLY.to_string())
        })();

        let _ = tx.send(result);
    })
    .map_err(|e| format!("Failed to schedule on main thread: {e}"))?;

    let previous = rx
        .await
        .map_err(|_| "Main thread task was dropped".to_string())??;

    if let Some(previous) = previous {
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(RESTORE_DELAY_MS)).await;
            let _ = handle.run_on_main_thread(move || {
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    // Re-read on the main thread and only hand `previous` back
                    // if our text is still the thing we'd be overwriting. This
                    // is what makes the restore safe whatever RESTORE_DELAY_MS
                    // is: too short and the user copied nothing yet, too long
                    // and they may have — either way we never clobber a copy
                    // that isn't ours.
                    if should_restore(cb.get_text().ok().as_deref(), &ours) {
                        let _ = cb.set_text(previous);
                    }
                }
            });
        });
    }

    Ok(())
}

#[tauri::command]
fn check_accessibility_permission() -> Result<bool, String> {
    Ok(permissions::is_accessibility_trusted())
}

#[tauri::command]
fn open_accessibility_settings() -> Result<(), String> {
    permissions::open_accessibility_settings()
}

#[tauri::command]
fn get_restore_clipboard(app: tauri::AppHandle) -> Result<bool, String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    let value =
        db::get_setting(&app_state.db_conn, RESTORE_CLIPBOARD_KEY).map_err(|e| e.to_string())?;
    Ok(value.as_deref() == Some("true"))
}

#[tauri::command]
fn set_restore_clipboard(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    db::set_setting(
        &app_state.db_conn,
        RESTORE_CLIPBOARD_KEY,
        if enabled { "true" } else { "false" },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn resize_window(app: tauri::AppHandle, width: f64, height: f64) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;

    // `set_size` alone anchors the top-left, so the recording card — six times
    // the height of the idle pill — would grow down and off the screen for
    // anyone who parked the pill near the bottom. Keep the widget's centre-x
    // and bottom edge fixed so the card unfurls in place, then clamp it back
    // inside the monitor's work area.
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let before = window
        .outer_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let origin = window
        .outer_position()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);

    window
        .set_size(tauri::Size::Logical(tauri::LogicalSize::new(width, height)))
        .map_err(|e| e.to_string())?;

    let mut x = origin.x + (before.width - width) / 2.0;
    let mut y = origin.y + (before.height - height);

    // A widget that cannot be found is worse than one that moved, so clamp to
    // the monitor it currently sits on whenever we can identify one.
    if let Ok(Some(monitor)) = window.current_monitor() {
        let area = monitor.size().to_logical::<f64>(monitor.scale_factor());
        let pos = monitor
            .position()
            .to_logical::<f64>(monitor.scale_factor());
        const MARGIN: f64 = 8.0;
        x = x.clamp(pos.x + MARGIN, (pos.x + area.width - width - MARGIN).max(pos.x + MARGIN));
        y = y.clamp(pos.y + MARGIN, (pos.y + area.height - height - MARGIN).max(pos.y + MARGIN));
    }

    window
        .set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)))
        .map_err(|e| e.to_string())
}

/// Live microphone loudness, 0.0..=1.0. Polled by the recording card so the
/// waveform reflects the user's actual voice; returns 0.0 when not recording.
#[tauri::command]
fn get_audio_level(state: tauri::State<'_, Mutex<AppState>>) -> Result<f32, String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    Ok(app_state
        .audio_capture
        .0
        .as_ref()
        .map(|c| f32::from_bits(c.level().load(std::sync::atomic::Ordering::Relaxed)))
        .unwrap_or(0.0))
}

#[tauri::command]
fn open_dashboard_window(app: tauri::AppHandle, section: Option<String>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("dashboard") {
        window.set_focus().map_err(|e| e.to_string())?;
        if let Some(ref s) = section {
            window
                .emit("navigate-section", s.clone())
                .map_err(|e: tauri::Error| e.to_string())?;
        }
        return Ok(());
    }

    let section_param = section.unwrap_or_default();
    let url = format!("index.html?window=dashboard&section={}", section_param);

    let _window =
        tauri::WebviewWindowBuilder::new(&app, "dashboard", tauri::WebviewUrl::App(url.into()))
            .title("Talx")
            .inner_size(720.0, 600.0)
            .decorations(true)
            .resizable(true)
            .center()
            .always_on_top(false)
            .build()
            .map_err(|e: tauri::Error| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn update_tray_status(app: tauri::AppHandle, status: String) -> Result<(), String> {
    let state = app.state::<Mutex<TrayState>>();
    let tray_state = state.lock().map_err(|e| e.to_string())?;

    let label = match status.as_str() {
        "idle" => "Status: Idle",
        "recording" => "Status: Recording...",
        "polishing" => "Status: Polishing...",
        "success" => "Status: Done",
        "result" => "Status: Ready",
        _ => "Status: Idle",
    };
    tray_state
        .status_item
        .set_text(label)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn update_tray_words(app: tauri::AppHandle, count: u32) -> Result<(), String> {
    let state = app.state::<Mutex<TrayState>>();
    let tray_state = state.lock().map_err(|e| e.to_string())?;

    let label = format!("Words today: {}", count);
    tray_state
        .words_item
        .set_text(&label)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn start_recording(app: tauri::AppHandle) -> Result<(), String> {
    let provider;
    let soniox_key;
    let asr_model_val;
    let mic_notice;

    // Check state and start audio capture
    {
        let state = app.state::<Mutex<AppState>>();
        let mut app_state = state.lock().map_err(|e| e.to_string())?;

        if !matches!(app_state.recording_status, RecordingStatus::Idle) {
            return Err("Already recording".to_string());
        }

        provider = app_state.asr_provider.clone();
        soniox_key = app_state.soniox_api_key.clone();
        asr_model_val = app_state.asr_model.clone();

        // Validate API key before starting
        match provider {
            asr::AsrProvider::Groq => {
                if app_state.groq_api_key.is_empty() {
                    return Err(
                        "Groq API key not configured. Set it in Settings > API Keys.".to_string(),
                    );
                }
            }
            asr::AsrProvider::Soniox => {
                if soniox_key.is_empty() {
                    return Err(
                        "Soniox API key not configured. Set it in Settings > API Keys.".to_string(),
                    );
                }
            }
        }

        // A failed settings read means "system default", never a failed recording.
        let preferred = db::get_setting(&app_state.db_conn, INPUT_DEVICE_KEY)
            .ok()
            .flatten()
            .unwrap_or_default();

        let mut capture = audio::AudioCapture::new()?;
        let (buffer, notice) = capture.start(&preferred)?;
        mic_notice = notice;
        app_state.audio_buffer = Some(buffer);
        app_state.audio_capture = SendableCapture(Some(capture));
        app_state.recording_status = RecordingStatus::Recording;
    }

    // Start Soniox streaming if that's the provider
    if let asr::AsrProvider::Soniox = provider {
        let (buffer, sample_rate, language) = {
            let state = app.state::<Mutex<AppState>>();
            let app_state = state.lock().map_err(|e| e.to_string())?;
            let buf = app_state.audio_buffer.clone().ok_or("No audio buffer")?;
            let sr = app_state
                .audio_capture
                .0
                .as_ref()
                .map(|c| c.sample_rate())
                .unwrap_or(16000);
            (buf, sr, app_state.language.clone())
        };

        let (partial_tx, mut partial_rx) = mpsc::unbounded_channel::<String>();

        let session = asr::soniox::SonioxSession::start(
            soniox_key,
            sample_rate,
            buffer,
            partial_tx,
            asr_model_val.clone(),
            &language,
        )
        .await?;

        {
            let state = app.state::<Mutex<AppState>>();
            let mut app_state = state.lock().map_err(|e| e.to_string())?;
            app_state.soniox_session = Some(session);
        }

        // Forward partial transcripts to frontend
        let app_handle = app.clone();
        tokio::spawn(async move {
            while let Some(partial) = partial_rx.recv().await {
                let _ = app_handle.emit("transcription-partial", &partial);
            }
        });
    }

    // Payload carries an optional mic-fallback notice; existing listeners ignore it.
    app.emit("recording-started", &mic_notice)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn list_input_devices() -> Result<Vec<audio::InputDevice>, String> {
    audio::list_input_devices()
}

#[tauri::command]
fn get_input_device(app: tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    Ok(db::get_setting(&app_state.db_conn, INPUT_DEVICE_KEY)
        .map_err(|e| e.to_string())?
        .unwrap_or_default())
}

#[tauri::command]
fn set_input_device(app: tauri::AppHandle, name: String) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    db::set_setting(&app_state.db_conn, INPUT_DEVICE_KEY, &name).map_err(|e| e.to_string())
}

/// Hand a finished transcription to the frontend, but only if the job that
/// produced it is still the current one — a `cancel_dictation` in the meantime
/// bumped `job_gen` and the result is dropped here rather than pasted into
/// whatever app the user was dictating into.
///
/// The emit deliberately happens while the lock is held. Releasing it first
/// would let a cancel slip in between the check and the emit, so the frontend
/// would see `dictation-cancelled` *before* `transcription-result` — and the
/// late result would start a fresh polish under the new generation and paste.
fn finish_job(app: &tauri::AppHandle, gen: u64, event: &str, payload: &str) {
    let state = app.state::<Mutex<AppState>>();
    let Ok(mut app_state) = state.lock() else {
        return;
    };
    if app_state.job_gen != gen {
        return;
    }
    app_state.recording_status = RecordingStatus::Result;
    let _ = app.emit(event, payload);
}

/// Abort an in-flight dictation and drop whatever it produces.
///
/// The spawned transcription task has no cancellation point inside its HTTP or
/// WebSocket call, so it still runs to completion — bumping `job_gen` is what
/// makes `finish_job` throw the result away, which is why nothing is pasted or
/// saved. Recording is also a valid state to cancel from (the Esc key), so the
/// capture and the streaming session are torn down here too.
#[tauri::command]
async fn cancel_dictation(app: tauri::AppHandle) -> Result<(), String> {
    {
        let state = app.state::<Mutex<AppState>>();
        let mut app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.job_gen = app_state.job_gen.wrapping_add(1);
        // Dropping the session closes the WebSocket. Usually already `None`:
        // `stop_recording` takes it before spawning.
        app_state.soniox_session = None;
        // Dropping the capture stops the cpal stream. This runs on the async
        // runtime (the command is `async`), which is the invariant
        // `SendableCapture`'s unsafe impl relies on.
        app_state.audio_capture = SendableCapture(None);
        app_state.audio_buffer = None;
        app_state.recording_status = RecordingStatus::Idle;
    }
    app.emit("dictation-cancelled", ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_recording(app: tauri::AppHandle) -> Result<(), String> {
    let provider;
    let samples;
    let sample_rate;
    let groq_key;
    let asr_model_val;
    let dictionary;
    let language;
    let gen;

    // Stop audio capture
    {
        let state = app.state::<Mutex<AppState>>();
        let mut app_state = state.lock().map_err(|e| e.to_string())?;

        if !matches!(app_state.recording_status, RecordingStatus::Recording) {
            return Err("Not recording".to_string());
        }

        app_state.recording_status = RecordingStatus::Transcribing;
        gen = app_state.job_gen;
        provider = app_state.asr_provider.clone();
        groq_key = app_state.groq_api_key.clone();
        asr_model_val = app_state.asr_model.clone();
        dictionary = app_state.dictionary.clone();
        language = app_state.language.clone();

        let (s, sr) = app_state
            .audio_capture
            .0
            .as_mut()
            .ok_or("No audio capture")?
            .stop();
        samples = s;
        sample_rate = sr;
        app_state.audio_capture = SendableCapture(None);
        app_state.audio_buffer = None;
    }

    let app_handle = app.clone();

    match provider {
        asr::AsrProvider::Groq => {
            tokio::spawn(async move {
                match asr::groq::transcribe(
                    &samples,
                    sample_rate,
                    &groq_key,
                    &asr_model_val,
                    &dictionary,
                    &language,
                )
                .await
                {
                    Ok(text) => finish_job(&app_handle, gen, "transcription-result", &text),
                    Err(e) => finish_job(&app_handle, gen, "recording-error", &e),
                }
            });
        }
        asr::AsrProvider::Soniox => {
            let mut session = {
                let state = app.state::<Mutex<AppState>>();
                let mut app_state = state.lock().map_err(|e| e.to_string())?;
                app_state.soniox_session.take()
            };

            tokio::spawn(async move {
                if let Some(ref mut session) = session {
                    match session.stop().await {
                        Ok(text) => finish_job(&app_handle, gen, "transcription-result", &text),
                        Err(e) => finish_job(&app_handle, gen, "recording-error", &e),
                    }
                }
            });
        }
    }

    Ok(())
}

#[tauri::command]
fn set_asr_provider(app: tauri::AppHandle, provider: String) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let mut app_state = state.lock().map_err(|e| e.to_string())?;

    app_state.asr_provider = match provider.as_str() {
        "soniox" => asr::AsrProvider::Soniox,
        _ => asr::AsrProvider::Groq,
    };

    Ok(())
}

#[tauri::command]
fn set_llm_provider(app: tauri::AppHandle, provider: String) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let mut app_state = state.lock().map_err(|e| e.to_string())?;

    app_state.llm_provider = match provider.as_str() {
        "openai" => llm::LlmProvider::OpenAi,
        _ => llm::LlmProvider::Groq,
    };

    Ok(())
}

#[tauri::command]
fn set_asr_model(app: tauri::AppHandle, model: String) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.asr_model = model;
    Ok(())
}

#[tauri::command]
fn set_llm_model(app: tauri::AppHandle, model: String) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.llm_model = model;
    Ok(())
}

#[derive(serde::Serialize)]
struct ModelInfo {
    id: String,
}

#[tauri::command]
async fn fetch_models(app: tauri::AppHandle, provider: String) -> Result<Vec<ModelInfo>, String> {
    let api_key = {
        let state = app.state::<Mutex<AppState>>();
        let app_state = state.lock().map_err(|e| e.to_string())?;
        match provider.as_str() {
            "groq" => app_state.groq_api_key.clone(),
            "soniox" => app_state.soniox_api_key.clone(),
            "llm_groq" => app_state.llm_groq_api_key.clone(),
            "llm_openai" => app_state.llm_openai_api_key.clone(),
            _ => return Err(format!("Unknown provider: {provider}")),
        }
    };

    // Soniox has no public model list API — offer the current real-time model
    // without requiring a key.
    if provider == "soniox" {
        return Ok(vec![ModelInfo {
            id: asr::soniox::DEFAULT_MODEL.to_string(),
        }]);
    }

    if api_key.is_empty() {
        return Err("API key not configured".to_string());
    }

    let client = reqwest::Client::new();

    match provider.as_str() {
        "groq" | "llm_groq" => {
            let response = client
                .get("https://api.groq.com/openai/v1/models")
                .header("Authorization", format!("Bearer {api_key}"))
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .map_err(|e| format!("Failed to fetch models: {e}"))?;

            if !response.status().is_success() {
                return Err(format!("API error ({})", response.status()));
            }

            let body: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("Parse error: {e}"))?;

            let models: Vec<ModelInfo> = body["data"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|m| {
                    let id = m["id"].as_str()?.to_string();
                    if provider == "groq" {
                        if id.contains("whisper") {
                            Some(ModelInfo { id })
                        } else {
                            None
                        }
                    } else {
                        if id.contains("whisper") {
                            None
                        } else {
                            Some(ModelInfo { id })
                        }
                    }
                })
                .collect();

            Ok(models)
        }
        "llm_openai" => {
            let response = client
                .get("https://api.openai.com/v1/models")
                .header("Authorization", format!("Bearer {api_key}"))
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .map_err(|e| format!("Failed to fetch models: {e}"))?;

            if !response.status().is_success() {
                return Err(format!("API error ({})", response.status()));
            }

            let body: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("Parse error: {e}"))?;

            let models: Vec<ModelInfo> = body["data"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|m| {
                    let id = m["id"].as_str()?.to_string();
                    if id.starts_with("gpt-") || id.starts_with("o") || id.starts_with("chatgpt-") {
                        Some(ModelInfo { id })
                    } else {
                        None
                    }
                })
                .collect();

            Ok(models)
        }
        _ => Err(format!("Unknown provider: {provider}")),
    }
}

#[tauri::command]
fn set_api_key(app: tauri::AppHandle, provider: String, key: String) -> Result<(), String> {
    match provider.as_str() {
        "groq" | "soniox" | "llm_groq" | "llm_openai" => {}
        _ => return Err(format!("Unknown provider: {provider}")),
    }

    let state = app.state::<Mutex<AppState>>();
    let mut app_state = state.lock().map_err(|e| e.to_string())?;

    let db_key = format!("api_key:{provider}");
    if key.is_empty() {
        db::delete_setting(&app_state.db_conn, &db_key).map_err(|e| e.to_string())?;
    } else {
        db::set_setting(&app_state.db_conn, &db_key, &key).map_err(|e| e.to_string())?;
    }

    match provider.as_str() {
        "groq" => app_state.groq_api_key = key,
        "soniox" => app_state.soniox_api_key = key,
        "llm_groq" => app_state.llm_groq_api_key = key,
        "llm_openai" => app_state.llm_openai_api_key = key,
        _ => unreachable!(),
    }

    Ok(())
}

#[tauri::command]
fn get_api_key(app: tauri::AppHandle, provider: String) -> Result<Option<String>, String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;

    let key = match provider.as_str() {
        "groq" => &app_state.groq_api_key,
        "soniox" => &app_state.soniox_api_key,
        "llm_groq" => &app_state.llm_groq_api_key,
        "llm_openai" => &app_state.llm_openai_api_key,
        _ => return Err(format!("Unknown provider: {provider}")),
    };

    if key.is_empty() {
        Ok(None)
    } else {
        Ok(Some(key.clone()))
    }
}

#[tauri::command]
fn delete_api_key(app: tauri::AppHandle, provider: String) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let mut app_state = state.lock().map_err(|e| e.to_string())?;

    let db_key = format!("api_key:{provider}");
    db::delete_setting(&app_state.db_conn, &db_key).map_err(|e| e.to_string())?;

    match provider.as_str() {
        "groq" => app_state.groq_api_key = String::new(),
        "soniox" => app_state.soniox_api_key = String::new(),
        "llm_groq" => app_state.llm_groq_api_key = String::new(),
        "llm_openai" => app_state.llm_openai_api_key = String::new(),
        _ => return Err(format!("Unknown provider: {provider}")),
    }

    Ok(())
}

#[tauri::command]
async fn test_api_key(provider: String, key: String) -> Result<(), String> {
    if key.is_empty() {
        return Err("API key is empty".to_string());
    }

    let client = reqwest::Client::new();

    match provider.as_str() {
        "groq" | "llm_groq" => {
            let response = client
                .get("https://api.groq.com/openai/v1/models")
                .header("Authorization", format!("Bearer {key}"))
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .map_err(|e| format!("Connection failed: {e}"))?;

            if !response.status().is_success() {
                return Err(format!("Invalid key ({})", response.status()));
            }
        }
        "soniox" => {
            use futures_util::{SinkExt, StreamExt};
            use tokio_tungstenite::tungstenite::Message;

            let (ws_stream, _) = tokio_tungstenite::connect_async(asr::soniox::ENDPOINT)
                .await
                .map_err(|e| format!("Connection failed: {e}"))?;

            let (mut ws_write, mut ws_read) = ws_stream.split();

            let config =
                asr::soniox::SonioxConfig::new(key, 16000, asr::soniox::DEFAULT_MODEL, "");
            let config_json =
                serde_json::to_string(&config).map_err(|e| format!("Config error: {e}"))?;
            ws_write
                .send(Message::Text(config_json.into()))
                .await
                .map_err(|e| format!("Failed to send config: {e}"))?;

            // The server stays silent after a valid config until audio arrives,
            // so end the stream immediately: a bad key answers with an error
            // message, a good one with `finished`.
            ws_write
                .send(Message::Text(String::new().into()))
                .await
                .map_err(|e| format!("Failed to send end-of-audio: {e}"))?;

            let response = tokio::time::timeout(std::time::Duration::from_secs(5), ws_read.next())
                .await
                .map_err(|_| "Connection timed out".to_string())?;

            let result = match response {
                Some(Ok(Message::Text(text))) => match asr::soniox::error_in(&text) {
                    Some(err) => Err(format!("Invalid key: {err}")),
                    None => Ok(()),
                },
                Some(Ok(Message::Close(frame))) => {
                    let reason = frame.map(|f| f.reason.to_string()).unwrap_or_default();
                    Err(format!("Connection rejected: {reason}"))
                }
                Some(Err(e)) => Err(format!("WebSocket error: {e}")),
                None => Err("Connection closed unexpectedly".to_string()),
                _ => Ok(()),
            };

            let _ = ws_write.close().await;
            result?;
        }
        "llm_openai" => {
            let response = client
                .get("https://api.openai.com/v1/models")
                .header("Authorization", format!("Bearer {key}"))
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .map_err(|e| format!("Connection failed: {e}"))?;

            if !response.status().is_success() {
                return Err(format!("Invalid key ({})", response.status()));
            }
        }
        _ => return Err(format!("Unknown provider: {provider}")),
    }

    Ok(())
}

#[tauri::command]
fn set_hotkey(app: tauri::AppHandle, hotkey: String) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let old_hotkey = {
        let state = app.state::<Mutex<AppState>>();
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.current_hotkey.clone()
    };

    // Unregister old shortcut
    app.global_shortcut()
        .unregister(old_hotkey.as_str())
        .map_err(|e| format!("Failed to unregister old hotkey: {e}"))?;

    // Register new shortcut
    if let Err(e) = register_hotkey(&app, &hotkey) {
        // Try to re-register old one on failure
        let _ = register_hotkey(&app, &old_hotkey);
        return Err(format!("Failed to register hotkey '{hotkey}': {e}"));
    }

    // Update state
    let state = app.state::<Mutex<AppState>>();
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.current_hotkey = hotkey;

    Ok(())
}

#[tauri::command]
fn get_hotkey(app: tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    Ok(app_state.current_hotkey.clone())
}

#[tauri::command]
fn dismiss_result(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    if !matches!(app_state.recording_status, RecordingStatus::Recording) {
        app_state.recording_status = RecordingStatus::Idle;
    }
    Ok(())
}

#[tauri::command]
async fn polish_transcript(
    app: tauri::AppHandle,
    text: String,
) -> Result<llm::PolishResult, String> {
    let (provider, api_key, llm_model, dictionary, style_rule) = {
        let state = app.state::<Mutex<AppState>>();
        let app_state = state.lock().map_err(|e| e.to_string())?;

        let key = match app_state.llm_provider {
            llm::LlmProvider::Groq => {
                if app_state.llm_groq_api_key.is_empty() {
                    return Err(
                        "Groq LLM API key not configured. Set it in Settings > API Keys."
                            .to_string(),
                    );
                }
                app_state.llm_groq_api_key.clone()
            }
            llm::LlmProvider::OpenAi => {
                if app_state.llm_openai_api_key.is_empty() {
                    return Err(
                        "OpenAI API key not configured. Set it in Settings > API Keys.".to_string(),
                    );
                }
                app_state.llm_openai_api_key.clone()
            }
        };

        // Style rule for the app captured at hotkey-down, never the app that is
        // frontmost right now (that could well be Talx).
        let style_rule = match (app_state.context_aware, app_state.target_app.as_ref()) {
            (true, Some(target)) => {
                let (category, rule) = context::classify(&target.bundle_id);
                println!(
                    "[talx][context] polishing for bundle={} category={category} rule={}",
                    target.bundle_id,
                    if rule.is_some() { "applied" } else { "none" }
                );
                rule
            }
            (true, None) => {
                println!("[talx][context] polishing with no captured target app");
                None
            }
            (false, _) => {
                println!("[talx][context] polishing with context-aware polish disabled");
                None
            }
        };

        (
            app_state.llm_provider.clone(),
            key,
            app_state.llm_model.clone(),
            app_state.dictionary.clone(),
            style_rule,
        )
    };

    llm::polish(&text, &provider, &api_key, &llm_model, &dictionary, style_rule).await
}

/// Reads the frontmost application and remembers it as the dictation target.
///
/// Must run at hotkey-down: by polish time Talx may be frontmost. A capture that
/// yields nothing usable (no frontmost app, no bundle id, or Talx itself) leaves
/// the previous target in place, so a hotkey pressed from inside Talx still
/// polishes for wherever the user last dictated.
fn capture_target_app(app: &tauri::AppHandle) {
    let handle = app.clone();
    let (tx, rx) = std::sync::mpsc::channel::<()>();

    // Cocoa accessors on the main thread, matching `paste_to_focused_app`.
    let dispatched = app.run_on_main_thread(move || {
        let own_bundle_id = handle.config().identifier.clone();
        if let Some(target) = context::frontmost_app(&own_bundle_id) {
            let state = handle.state::<Mutex<AppState>>();
            // Bound to a local so the guard drops before `state` does.
            let locked = state.lock();
            if let Ok(mut app_state) = locked {
                app_state.target_app = Some(target);
            }
        }
        let _ = tx.send(());
    });

    // Wait for the capture to land before the caller starts recording. Tauri
    // runs the closure inline when we are already on the main thread, so this
    // returns immediately in the common case; when the hotkey handler is on
    // another thread the closure is queued instead, and without this wait the
    // capture would race the rest of the key-down path.
    //
    // ponytail: bounded wait, not a handshake — a main thread busy for >150ms
    // loses the capture and polish falls back to the previous target, which
    // beats stalling the hotkey.
    if dispatched.is_ok() {
        let _ = rx.recv_timeout(std::time::Duration::from_millis(150));
    }
}

#[tauri::command]
fn get_context_aware(app: tauri::AppHandle) -> Result<bool, String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    Ok(app_state.context_aware)
}

#[tauri::command]
fn set_context_aware(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    db::set_setting(
        &app_state.db_conn,
        CONTEXT_AWARE_KEY,
        if enabled { "true" } else { "false" },
    )
    .map_err(|e| e.to_string())?;
    app_state.context_aware = enabled;
    Ok(())
}

#[tauri::command]
fn get_language(app: tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    Ok(app_state.language.clone())
}

/// Persist the recognition language. Normalizes first so an unusable code can
/// never sit in the DB and `get_language` always round-trips a real option.
#[tauri::command]
fn set_language(app: tauri::AppHandle, language: String) -> Result<(), String> {
    let normalized = normalize_language(&language);

    let state = app.state::<Mutex<AppState>>();
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    db::set_setting(&app_state.db_conn, LANGUAGE_KEY, &normalized).map_err(|e| e.to_string())?;
    app_state.language = normalized;
    Ok(())
}

#[tauri::command]
fn save_transcription(
    app: tauri::AppHandle,
    original_text: String,
    polished_text: String,
    word_count: u32,
    duration_secs: u32,
) -> Result<db::TranscriptionRow, String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    db::save(
        &app_state.db_conn,
        &original_text,
        &polished_text,
        word_count,
        duration_secs,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_transcriptions(
    app: tauri::AppHandle,
    limit: u32,
    offset: u32,
) -> Result<Vec<db::TranscriptionRow>, String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    db::get(&app_state.db_conn, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_transcriptions(
    app: tauri::AppHandle,
    query: String,
    limit: u32,
) -> Result<Vec<db::TranscriptionRow>, String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    db::search(&app_state.db_conn, &query, limit).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_transcription_timestamps(app: tauri::AppHandle, limit: u32) -> Result<Vec<i64>, String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    db::get_timestamps(&app_state.db_conn, limit).map_err(|e| e.to_string())
}

/// Write the whole history to `path` as CSV or Markdown. The frontend picks the
/// path with the save dialog, so this only serializes and writes.
#[tauri::command]
fn export_history(app: tauri::AppHandle, format: String, path: String) -> Result<(), String> {
    let rows = {
        let state = app.state::<Mutex<AppState>>();
        let app_state = state.lock().map_err(|e| e.to_string())?;
        export::fetch_all(&app_state.db_conn).map_err(|e| e.to_string())?
    };

    let contents = match format.as_str() {
        "csv" => export::to_csv(&rows),
        "md" => export::to_markdown(&rows),
        other => return Err(format!("Unsupported export format: {other}")),
    };

    std::fs::write(&path, contents).map_err(|e| format!("Failed to write {path}: {e}"))
}

#[tauri::command]
fn delete_transcription(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    db::delete(&app_state.db_conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_transcriptions(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    db::clear(&app_state.db_conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_dictionary(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    Ok(app_state.dictionary.clone())
}

/// Replace the whole dictionary. Persists to SQLite and updates the in-memory
/// copy so the next dictation picks it up without a restart.
#[tauri::command]
fn set_dictionary(app: tauri::AppHandle, terms: Vec<String>) -> Result<Vec<String>, String> {
    let normalized = dictionary::normalize(&terms);

    let state = app.state::<Mutex<AppState>>();
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    dictionary::save(&app_state.db_conn, &normalized)?;
    app_state.dictionary = normalized.clone();

    Ok(normalized)
}

#[tauri::command]
fn get_today_stats(app: tauri::AppHandle, since_ms: i64) -> Result<db::TodayStats, String> {
    let state = app.state::<Mutex<AppState>>();
    let app_state = state.lock().map_err(|e| e.to_string())?;
    db::get_today_stats(&app_state.db_conn, since_ms).map_err(|e| e.to_string())
}

fn register_hotkey(app: &tauri::AppHandle, shortcut: &str) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let app_handle = app.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, {
            move |_app, _shortcut, event| {
                let state = app_handle.state::<Mutex<AppState>>();

                let (status, hold_mode) = {
                    let Ok(app_state) = state.lock() else {
                        return;
                    };
                    let s = match app_state.recording_status {
                        RecordingStatus::Idle => 0u8,
                        RecordingStatus::Recording => 1,
                        _ => 2,
                    };
                    (s, app_state.hold_mode)
                };

                match event.state {
                    ShortcutState::Pressed => {
                        if status == 0 {
                            // Idle → start recording, record key-down time.
                            // Capture the dictation target FIRST: this is the
                            // last moment the user's app is guaranteed frontmost.
                            capture_target_app(&app_handle);
                            if let Ok(mut app_state) = state.lock() {
                                app_state.key_down_time = Some(std::time::Instant::now());
                                app_state.hold_mode = false;
                            }
                            let handle = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = start_recording(handle.clone()).await {
                                    let _ = handle.emit("recording-error", &e);
                                }
                            });
                        } else if status == 1 && !hold_mode {
                            // Toggle-started recording: short press again to stop
                            let handle = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = stop_recording(handle.clone()).await {
                                    let _ = handle.emit("recording-error", &e);
                                }
                            });
                        } else if status == 2 {
                            // Transcribing or polishing: the recording is already
                            // committed, so a press here is the only escape hatch
                            // before the transcript gets pasted. `status == 2` also
                            // covers the post-paste result card — the frontend
                            // decides whether to label that a cancel.
                            let handle = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = cancel_dictation(handle.clone()).await {
                                    let _ = handle.emit("recording-error", &e);
                                }
                            });
                        }
                    }
                    ShortcutState::Released => {
                        if status == 1 {
                            // Check if this was a hold (≥400ms)
                            let is_hold = if let Ok(mut app_state) = state.lock() {
                                if let Some(down_time) = app_state.key_down_time {
                                    if down_time.elapsed() >= std::time::Duration::from_millis(400)
                                    {
                                        app_state.hold_mode = true;
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                            if is_hold {
                                let handle = app_handle.clone();
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = stop_recording(handle.clone()).await {
                                        let _ = handle.emit("recording-error", &e);
                                    }
                                });
                            }
                        }
                    }
                }
            }
        })
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let status_item = tauri::menu::MenuItemBuilder::new("Status: Idle")
                .id("status")
                .enabled(false)
                .build(app)?;

            let open_dashboard = tauri::menu::MenuItemBuilder::new("Open Dashboard")
                .id("open-dashboard")
                .build(app)?;

            let open_settings = tauri::menu::MenuItemBuilder::new("Settings")
                .id("open-settings")
                .build(app)?;

            let words_item = tauri::menu::MenuItemBuilder::new("Words today: 0")
                .id("words-today")
                .enabled(false)
                .build(app)?;

            let quit_item = tauri::menu::MenuItemBuilder::new("Quit Talx")
                .id("quit")
                .build(app)?;

            let menu = tauri::menu::MenuBuilder::new(app)
                .item(&status_item)
                .separator()
                .item(&open_dashboard)
                .item(&open_settings)
                .separator()
                .item(&words_item)
                .separator()
                .item(&quit_item)
                .build()?;

            app.manage(Mutex::new(TrayState {
                status_item: status_item.clone(),
                words_item: words_item.clone(),
            }));

            let app_data_dir = app.path().app_data_dir()?;
            let db_conn =
                db::init_db(&app_data_dir).map_err(|e| format!("Failed to init database: {e}"))?;

            app.manage(Mutex::new(AppState {
                recording_status: RecordingStatus::Idle,
                audio_capture: SendableCapture(None),
                soniox_session: None,
                asr_provider: asr::AsrProvider::Groq,
                asr_model: "whisper-large-v3-turbo".to_string(),
                audio_buffer: None,
                groq_api_key: String::new(),
                soniox_api_key: String::new(),
                llm_provider: llm::LlmProvider::default(),
                llm_model: "llama-3.3-70b-versatile".to_string(),
                llm_groq_api_key: String::new(),
                llm_openai_api_key: String::new(),
                current_hotkey: default_hotkey().to_string(),
                dictionary: Vec::new(),
                db_conn,
                key_down_time: None,
                hold_mode: false,
                target_app: None,
                context_aware: true,
                language: String::new(),
                job_gen: 0,
            }));

            // Context-aware polish: default ON, only an explicit "false" disables it.
            {
                let state = app.state::<Mutex<AppState>>();
                // Bound to a local so the guard drops before `state` does.
                let locked = state.lock();
                if let Ok(mut app_state) = locked {
                    let stored = db::get_setting(&app_state.db_conn, CONTEXT_AWARE_KEY)
                        .ok()
                        .flatten();
                    app_state.context_aware = stored.as_deref() != Some("false");

                    // Recognition language: absent or unreadable = auto-detect.
                    let stored = db::get_setting(&app_state.db_conn, LANGUAGE_KEY)
                        .ok()
                        .flatten()
                        .unwrap_or_default();
                    app_state.language = normalize_language(&stored);
                }
            }

            // One-time migration: pull API keys from macOS Keychain (legacy keyring crate)
            // into SQLite, then never run again.
            #[cfg(target_os = "macos")]
            {
                let state = app.state::<Mutex<AppState>>();
                if let Ok(app_state) = state.lock() {
                    let migrated = db::get_setting(&app_state.db_conn, "keychain_migrated")
                        .ok()
                        .flatten()
                        .is_some();

                    if !migrated {
                        let providers = ["groq", "soniox", "llm_groq", "llm_openai"];
                        for provider in providers {
                            let output = std::process::Command::new("security")
                                .args([
                                    "find-generic-password",
                                    "-s",
                                    "com.talx.app",
                                    "-a",
                                    provider,
                                    "-w",
                                ])
                                .output();
                            if let Ok(output) = output {
                                if output.status.success() {
                                    let key =
                                        String::from_utf8_lossy(&output.stdout).trim().to_string();
                                    if !key.is_empty() {
                                        let db_key = format!("api_key:{provider}");
                                        let _ = db::set_setting(&app_state.db_conn, &db_key, &key);
                                    }
                                }
                            }
                        }
                        let _ = db::set_setting(&app_state.db_conn, "keychain_migrated", "1");
                    }
                };
            }

            // Load API keys from database (with env var fallback for dev)
            {
                let state = app.state::<Mutex<AppState>>();
                if let Ok(mut app_state) = state.lock() {
                    let providers = ["groq", "soniox", "llm_groq", "llm_openai"];
                    let env_vars = [
                        ("groq", "GROQ_API_KEY"),
                        ("soniox", "SONIOX_API_KEY"),
                        ("llm_groq", "GROQ_LLM_API_KEY"),
                        ("llm_openai", "OPENAI_API_KEY"),
                    ];

                    for provider in providers {
                        // Check env var first (dev mode override)
                        let env_key = env_vars
                            .iter()
                            .find(|(p, _)| *p == provider)
                            .map(|(_, v)| *v);
                        let from_env = env_key
                            .and_then(|var| std::env::var(var).ok())
                            .filter(|v| !v.is_empty());

                        let key = from_env.or_else(|| {
                            let db_key = format!("api_key:{provider}");
                            db::get_setting(&app_state.db_conn, &db_key).ok().flatten()
                        });

                        if let Some(key) = key {
                            match provider {
                                "groq" => app_state.groq_api_key = key,
                                "soniox" => app_state.soniox_api_key = key,
                                "llm_groq" => app_state.llm_groq_api_key = key,
                                "llm_openai" => app_state.llm_openai_api_key = key,
                                _ => {}
                            }
                        }
                    }
                };
            }

            // Load the personal dictionary from the settings table
            {
                let state = app.state::<Mutex<AppState>>();
                if let Ok(mut app_state) = state.lock() {
                    let terms = dictionary::load(&app_state.db_conn);
                    app_state.dictionary = terms;
                };
            }

            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

            let _tray = tauri::tray::TrayIconBuilder::with_id("main-tray")
                .menu(&menu)
                .icon(icon)
                .tooltip("Talx")
                .on_menu_event(
                    |app_handle: &tauri::AppHandle, event: tauri::menu::MenuEvent| match event
                        .id()
                        .as_ref()
                    {
                        "open-dashboard" => {
                            let _ = open_dashboard_window(app_handle.clone(), None);
                        }
                        "open-settings" => {
                            let _ = open_dashboard_window(
                                app_handle.clone(),
                                Some("settings".to_string()),
                            );
                        }
                        "quit" => {
                            app_handle.exit(0);
                        }
                        _ => {}
                    },
                )
                .build(app)?;

            // Make the webview background transparent on macOS
            #[cfg(target_os = "macos")]
            {
                #[allow(deprecated)]
                fn clear_window_bg(window: &tauri::WebviewWindow) -> Result<(), String> {
                    use cocoa::appkit::{NSColor, NSWindow};
                    use cocoa::base::{id, nil};
                    let ns_win = window.ns_window().map_err(|e| e.to_string())?;
                    unsafe {
                        let ns_win = ns_win as id;
                        ns_win.setBackgroundColor_(NSColor::clearColor(nil));
                    }
                    Ok(())
                }
                if let Some(main_window) = app.get_webview_window("main") {
                    let _ = clear_window_bg(&main_window);
                }
            }

            // Register global hotkey (⌥Space)
            register_hotkey(app.handle(), default_hotkey())?;

            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                // Small delay to let the app fully initialize
                std::thread::sleep(std::time::Duration::from_millis(300));

                let _ = open_dashboard_window(app_handle.clone(), Some("settings".to_string()));

                // Show widget after app is ready
                std::thread::sleep(std::time::Duration::from_millis(500));
                if let Some(main_window) = app_handle.get_webview_window("main") {
                    let _ = main_window.show();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            paste_to_focused_app,
            copy_to_clipboard,
            check_accessibility_permission,
            open_accessibility_settings,
            get_restore_clipboard,
            set_restore_clipboard,
            resize_window,
            get_audio_level,
            open_dashboard_window,
            update_tray_status,
            update_tray_words,
            start_recording,
            stop_recording,
            list_input_devices,
            get_input_device,
            set_input_device,
            set_asr_provider,
            set_llm_provider,
            set_asr_model,
            set_llm_model,
            fetch_models,
            set_api_key,
            get_api_key,
            delete_api_key,
            test_api_key,
            set_hotkey,
            get_hotkey,
            dismiss_result,
            cancel_dictation,
            polish_transcript,
            save_transcription,
            get_transcriptions,
            search_transcriptions,
            get_transcription_timestamps,
            export_history,
            delete_transcription,
            clear_transcriptions,
            get_today_stats,
            get_dictionary,
            set_dictionary,
            get_context_aware,
            set_context_aware,
            get_language,
            set_language
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            match event {
                tauri::RunEvent::ExitRequested { api, .. } => {
                    // Prevent app from quitting when windows close — this is a tray app
                    api.prevent_exit();
                }
                #[cfg(target_os = "macos")]
                tauri::RunEvent::Reopen { .. } => {
                    let _ =
                        open_dashboard_window(_app_handle.clone(), Some("settings".to_string()));
                }
                _ => {}
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_restore_only_when_our_text_is_still_on_the_clipboard() {
        // The clipboard still holds what we wrote — safe to hand `previous` back.
        assert!(should_restore(Some("hello world"), "hello world"));
        // The user copied something during the delay — theirs wins.
        assert!(!should_restore(
            Some("something the user copied"),
            "hello world"
        ));
        // Nothing readable as text (image, file, cleared): no text of ours to
        // overwrite, so leave the clipboard alone.
        assert!(!should_restore(None, "hello world"));
        assert!(!should_restore(Some(""), "hello world"));
        // An empty transcript would otherwise match an empty clipboard.
        assert!(!should_restore(Some(""), ""));
    }
}
