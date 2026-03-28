use std::sync::Mutex;
use tauri::Manager;

struct TrayState {
    status_item: tauri::menu::MenuItem<tauri::Wry>,
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
fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let _window = tauri::WebviewWindowBuilder::new(
        &app,
        "settings",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("Talx Settings")
    .inner_size(720.0, 520.0)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let status_item = tauri::menu::MenuItemBuilder::new("Status: Idle")
                .id("status")
                .enabled(false)
                .build(app)?;

            let open_settings = tauri::menu::MenuItemBuilder::new("Open Settings")
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
                .item(&open_settings)
                .separator()
                .item(&words_item)
                .separator()
                .item(&quit_item)
                .build()?;

            app.manage(Mutex::new(TrayState {
                status_item: status_item.clone(),
            }));

            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
                .expect("failed to load tray icon");

            let _tray = tauri::tray::TrayIconBuilder::with_id("main-tray")
                .menu(&menu)
                .icon(icon)
                .tooltip("Talx")
                .on_menu_event(|app_handle: &tauri::AppHandle, event: tauri::menu::MenuEvent| {
                    match event.id().as_ref() {
                        "open-settings" => {
                            let _ = open_settings_window(app_handle.clone());
                        }
                        "quit" => {
                            app_handle.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Open settings dashboard + show widget with delay
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                // Small delay to let the app fully initialize
                std::thread::sleep(std::time::Duration::from_millis(300));

                // Open settings (dashboard) first
                let _ = open_settings_window(app_handle.clone());

                // Show widget after dashboard is up
                std::thread::sleep(std::time::Duration::from_millis(500));
                if let Some(main_window) = app_handle.get_webview_window("main") {
                    let _ = main_window.show();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            resize_window,
            open_settings_window,
            update_tray_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
