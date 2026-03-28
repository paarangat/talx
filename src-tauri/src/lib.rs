use std::sync::Mutex;
use tauri::Emitter;
use tauri::Manager;

struct TrayState {
    status_item: tauri::menu::MenuItem<tauri::Wry>,
    words_item: tauri::menu::MenuItem<tauri::Wry>,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
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
            update_tray_words
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Reopen { .. } = event {
                let _ = open_dashboard_window(app_handle.clone(), None);
            }
        });
}
