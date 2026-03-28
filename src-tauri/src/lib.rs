use std::sync::Mutex;
use tauri::Emitter;
use tauri::Manager;

mod asr;
mod audio;
mod llm;

use tokio::sync::mpsc;

struct TrayState {
    status_item: tauri::menu::MenuItem<tauri::Wry>,
    words_item: tauri::menu::MenuItem<tauri::Wry>,
}

enum RecordingStatus {
    Idle,
    Recording,
    Transcribing,
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
    audio_buffer: Option<std::sync::Arc<Mutex<Vec<i16>>>>,
    groq_api_key: String,
    soniox_api_key: String,
    llm_provider: llm::LlmProvider,
    llm_groq_api_key: String,
    llm_openai_api_key: String,
    current_hotkey: String,
}

#[tauri::command]
fn resize_window(app: tauri::AppHandle, width: f64, height: f64) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;
    let size = tauri::LogicalSize::new(width, height);
    window
        .set_size(tauri::Size::Logical(size))
        .map_err(|e| e.to_string())
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

    let _window = tauri::WebviewWindowBuilder::new(
        &app,
        "dashboard",
        tauri::WebviewUrl::App(url.into()),
    )
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

    // Check state and start audio capture
    {
        let state = app.state::<Mutex<AppState>>();
        let mut app_state = state.lock().map_err(|e| e.to_string())?;

        if !matches!(app_state.recording_status, RecordingStatus::Idle) {
            return Err("Already recording".to_string());
        }

        provider = app_state.asr_provider.clone();
        soniox_key = app_state.soniox_api_key.clone();

        // Validate API key before starting
        match provider {
            asr::AsrProvider::Groq => {
                if app_state.groq_api_key.is_empty() {
                    return Err("Groq API key not configured. Set it in Settings > API Keys.".to_string());
                }
            }
            asr::AsrProvider::Soniox => {
                if soniox_key.is_empty() {
                    return Err("Soniox API key not configured. Set it in Settings > API Keys.".to_string());
                }
            }
        }

        let mut capture = audio::AudioCapture::new()?;
        let buffer = capture.start()?;
        app_state.audio_buffer = Some(buffer);
        app_state.audio_capture = SendableCapture(Some(capture));
        app_state.recording_status = RecordingStatus::Recording;
    }

    // Start Soniox streaming if that's the provider
    if let asr::AsrProvider::Soniox = provider {
        let (buffer, sample_rate) = {
            let state = app.state::<Mutex<AppState>>();
            let app_state = state.lock().map_err(|e| e.to_string())?;
            let buf = app_state.audio_buffer.clone().ok_or("No audio buffer")?;
            let sr = app_state.audio_capture.0.as_ref().map(|c| c.sample_rate()).unwrap_or(16000);
            (buf, sr)
        };

        let (partial_tx, mut partial_rx) = mpsc::unbounded_channel::<String>();

        let session = asr::soniox::SonioxSession::start(
            soniox_key,
            sample_rate,
            buffer,
            partial_tx,
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

    app.emit("recording-started", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn stop_recording(app: tauri::AppHandle) -> Result<(), String> {
    let provider;
    let samples;
    let sample_rate;
    let groq_key;

    // Stop audio capture
    {
        let state = app.state::<Mutex<AppState>>();
        let mut app_state = state.lock().map_err(|e| e.to_string())?;

        if !matches!(app_state.recording_status, RecordingStatus::Recording) {
            return Err("Not recording".to_string());
        }

        app_state.recording_status = RecordingStatus::Transcribing;
        provider = app_state.asr_provider.clone();
        groq_key = app_state.groq_api_key.clone();

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
                match asr::groq::transcribe(&samples, sample_rate, &groq_key).await {
                    Ok(text) => {
                        let _ = app_handle.emit("transcription-result", &text);
                    }
                    Err(e) => {
                        let _ = app_handle.emit("recording-error", &e);
                    }
                }
                let state = app_handle.state::<Mutex<AppState>>();
                if let Ok(mut app_state) = state.lock() {
                    app_state.recording_status = RecordingStatus::Result;
                };
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
                        Ok(text) => {
                            let _ = app_handle.emit("transcription-result", &text);
                        }
                        Err(e) => {
                            let _ = app_handle.emit("recording-error", &e);
                        }
                    }
                }
                let state = app_handle.state::<Mutex<AppState>>();
                if let Ok(mut app_state) = state.lock() {
                    app_state.recording_status = RecordingStatus::Result;
                };
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
fn set_api_key(app: tauri::AppHandle, provider: String, key: String) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let mut app_state = state.lock().map_err(|e| e.to_string())?;

    match provider.as_str() {
        "groq" => app_state.groq_api_key = key,
        "soniox" => app_state.soniox_api_key = key,
        "llm_groq" => app_state.llm_groq_api_key = key,
        "llm_openai" => app_state.llm_openai_api_key = key,
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
            use tokio_tungstenite::tungstenite::Message;
            use futures_util::{SinkExt, StreamExt};

            let (ws_stream, _) = tokio_tungstenite::connect_async(
                "wss://api.soniox.com/transcribe-websocket",
            )
            .await
            .map_err(|e| format!("Connection failed: {e}"))?;

            let (mut ws_write, mut ws_read) = ws_stream.split();

            let config = serde_json::json!({
                "api_key": key,
                "sample_rate_hertz": 16000,
                "audio_format": "pcm_s16le",
                "model": "soniox-v2",
                "include_nonfinal": false
            });

            ws_write
                .send(Message::Text(config.to_string().into()))
                .await
                .map_err(|e| format!("Failed to send config: {e}"))?;

            // Read one response to check for auth errors
            let response = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                ws_read.next(),
            )
            .await
            .map_err(|_| "Connection timed out".to_string())?;

            let result = match response {
                Some(Ok(Message::Text(text))) => {
                    let text_str: &str = &text;
                    let lower = text_str.to_lowercase();
                    if lower.contains("error") || lower.contains("unauthorized") || lower.contains("invalid") {
                        Err(format!("Invalid key: {text_str}"))
                    } else {
                        Ok(())
                    }
                }
                Some(Ok(Message::Close(frame))) => {
                    let reason = frame.map(|f| f.reason.to_string()).unwrap_or_default();
                    Err(format!("Connection rejected: {reason}"))
                }
                Some(Err(e)) => {
                    Err(format!("WebSocket error: {e}"))
                }
                None => {
                    Err("Connection closed unexpectedly".to_string())
                }
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
    if matches!(app_state.recording_status, RecordingStatus::Result) {
        app_state.recording_status = RecordingStatus::Idle;
    }
    Ok(())
}

#[tauri::command]
async fn polish_transcript(app: tauri::AppHandle, text: String) -> Result<llm::PolishResult, String> {
    let (provider, api_key) = {
        let state = app.state::<Mutex<AppState>>();
        let app_state = state.lock().map_err(|e| e.to_string())?;

        let key = match app_state.llm_provider {
            llm::LlmProvider::Groq => {
                if app_state.llm_groq_api_key.is_empty() {
                    return Err("Groq LLM API key not configured. Set it in Settings > API Keys.".to_string());
                }
                app_state.llm_groq_api_key.clone()
            }
            llm::LlmProvider::OpenAi => {
                if app_state.llm_openai_api_key.is_empty() {
                    return Err("OpenAI API key not configured. Set it in Settings > API Keys.".to_string());
                }
                app_state.llm_openai_api_key.clone()
            }
        };

        (app_state.llm_provider.clone(), key)
    };

    llm::polish(&text, &provider, &api_key).await
}

fn register_hotkey(
    app: &tauri::AppHandle,
    shortcut: &str,
) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let app_handle = app.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, {
            move |_app, _shortcut, event| {
                if event.state != ShortcutState::Pressed {
                    return;
                }

                let is_recording = {
                    let state = app_handle.state::<Mutex<AppState>>();
                    let result = if let Ok(app_state) = state.lock() {
                        matches!(app_state.recording_status, RecordingStatus::Recording)
                    } else {
                        false
                    };
                    result
                };

                let handle = app_handle.clone();
                if is_recording {
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = stop_recording(handle.clone()).await {
                            let _ = handle.emit("recording-error", &e);
                        }
                    });
                } else {
                    let is_idle = {
                        let state = app_handle.state::<Mutex<AppState>>();
                        let result = if let Ok(app_state) = state.lock() {
                            matches!(app_state.recording_status, RecordingStatus::Idle)
                        } else {
                            false
                        };
                        result
                    };
                    if is_idle {
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = start_recording(handle.clone()).await {
                                let _ = handle.emit("recording-error", &e);
                            }
                        });
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

            app.manage(Mutex::new(AppState {
                recording_status: RecordingStatus::Idle,
                audio_capture: SendableCapture(None),
                soniox_session: None,
                asr_provider: asr::AsrProvider::Groq,
                audio_buffer: None,
                groq_api_key: String::new(),
                soniox_api_key: String::new(),
                llm_provider: llm::LlmProvider::default(),
                llm_groq_api_key: String::new(),
                llm_openai_api_key: String::new(),
                current_hotkey: "alt+space".to_string(),
            }));

            // Load API keys from environment variables for dev
            {
                let state = app.state::<Mutex<AppState>>();
                if let Ok(mut app_state) = state.lock() {
                    if let Ok(key) = std::env::var("GROQ_API_KEY") {
                        app_state.groq_api_key = key;
                    }
                    if let Ok(key) = std::env::var("SONIOX_API_KEY") {
                        app_state.soniox_api_key = key;
                    }
                    if let Ok(key) = std::env::var("GROQ_API_KEY") {
                        if app_state.llm_groq_api_key.is_empty() {
                            app_state.llm_groq_api_key = key;
                        }
                    }
                    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
                        app_state.llm_openai_api_key = key;
                    }
                };
            }

            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

            let _tray = tauri::tray::TrayIconBuilder::with_id("main-tray")
                .menu(&menu)
                .icon(icon)
                .tooltip("Talx")
                .on_menu_event(|app_handle: &tauri::AppHandle, event: tauri::menu::MenuEvent| {
                    match event.id().as_ref() {
                        "open-dashboard" => {
                            let _ = open_dashboard_window(app_handle.clone(), None);
                        }
                        "open-settings" => {
                            let _ = open_dashboard_window(app_handle.clone(), Some("settings".to_string()));
                        }
                        "quit" => {
                            app_handle.exit(0);
                        }
                        _ => {}
                    }
                })
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
            register_hotkey(app.handle(), "alt+space")?;

            // Check if this is the first launch
            let app_data_dir = app.path().app_data_dir()?;
            let first_launch_flag = app_data_dir.join(".launched");
            let is_first_launch = !first_launch_flag.exists();

            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                // Small delay to let the app fully initialize
                std::thread::sleep(std::time::Duration::from_millis(300));

                // Open settings only on first launch
                if is_first_launch {
                    let _ = open_dashboard_window(app_handle.clone(), None);
                    let _ = std::fs::create_dir_all(&app_data_dir);
                    let _ = std::fs::write(&first_launch_flag, b"");
                }

                // Show widget after app is ready
                std::thread::sleep(std::time::Duration::from_millis(500));
                if let Some(main_window) = app_handle.get_webview_window("main") {
                    let _ = main_window.show();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            resize_window,
            open_dashboard_window,
            update_tray_status,
            update_tray_words,
            start_recording,
            stop_recording,
            set_asr_provider,
            set_llm_provider,
            set_api_key,
            test_api_key,
            set_hotkey,
            get_hotkey,
            dismiss_result,
            polish_transcript
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Reopen { .. } = event {
                let _ = open_dashboard_window(app_handle.clone(), None);
            }
        });
}
