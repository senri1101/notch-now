use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{App, AppHandle, Emitter, Manager, PhysicalPosition, Position, WebviewWindow};

const HOTKEY_INTERVAL_MS: u64 = 300;

#[derive(Default)]
struct HotkeyState {
    inner: Mutex<HotkeyStateInner>,
}

#[derive(Default)]
struct HotkeyStateInner {
    last_pressed_at: Option<Instant>,
    sequence: u64,
}

#[tauri::command]
fn set_click_through(window: WebviewWindow, enable: bool) -> Result<(), String> {
    window
        .set_ignore_cursor_events(enable)
        .map_err(|error| error.to_string())
}

// D1: File-based persistence
#[tauri::command]
fn read_text(app: AppHandle) -> Result<String, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let file_path = data_dir.join("task.txt");
    if !file_path.exists() {
        return Ok(String::new());
    }
    std::fs::read_to_string(&file_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_text(app: AppHandle, text: String) -> Result<(), String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    let file_path = data_dir.join("task.txt");
    std::fs::write(&file_path, text).map_err(|e| e.to_string())
}

fn emit_mode(app: &AppHandle, mode: &str) {
    if let Some(window) = app.get_webview_window("main") {
        match mode {
            "edit" => {
                let _ = window.set_ignore_cursor_events(false);
            }
            _ => {
                let _ = window.set_ignore_cursor_events(true);
            }
        }
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("mode", mode.to_string());
    }
}

fn handle_hotkey_pressed(app: &AppHandle) {
    let interval = Duration::from_millis(HOTKEY_INTERVAL_MS);
    let mut should_emit_edit = false;
    let token = {
        let state = app.state::<HotkeyState>();
        let mut inner = state.inner.lock().expect("hotkey state lock poisoned");
        let now = Instant::now();
        let is_double_press = inner
            .last_pressed_at
            .map(|last| now.duration_since(last) <= interval)
            .unwrap_or(false);

        inner.sequence = inner.sequence.wrapping_add(1);
        let sequence = inner.sequence;

        if is_double_press {
            inner.last_pressed_at = None;
            should_emit_edit = true;
        } else {
            inner.last_pressed_at = Some(now);
        }

        sequence
    };

    if should_emit_edit {
        emit_mode(app, "edit");
        return;
    }

    let app_handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(interval);

        let should_act = {
            let state = app_handle.state::<HotkeyState>();
            let mut inner = state.inner.lock().expect("hotkey state lock poisoned");
            if inner.sequence == token {
                inner.last_pressed_at = None;
                true
            } else {
                false
            }
        };

        if should_act {
            if let Some(window) = app_handle.get_webview_window("main") {
                let is_visible = window.is_visible().unwrap_or(true);
                if is_visible {
                    // Window visible → hide
                    let _ = window.set_ignore_cursor_events(true);
                    let _ = window.hide();
                } else {
                    // Window hidden → show with emphasis
                    let _ = window.show();
                    let _ = window.set_focus();
                    let _ = window.emit("mode", "emphasis".to_string());
                }
            }
        }
    });
}

fn place_window_to_top_left(window: &WebviewWindow) {
    let Some(monitor) = window.current_monitor().ok().flatten() else {
        return;
    };
    let monitor_position = monitor.position();
    let top_left_x = monitor_position.x + 8;
    let top_left_y = monitor_position.y + 6;

    let _ = window.set_position(Position::Physical(PhysicalPosition::new(
        top_left_x, top_left_y,
    )));
}

// A3: System tray icon
fn setup_tray(app: &App) -> tauri::Result<()> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::TrayIconBuilder;

    let quit_item = MenuItemBuilder::with_id("quit", "Quit doing-now").build(app)?;
    let menu = MenuBuilder::new(app).item(&quit_item).build()?;

    if let Some(icon) = app.default_window_icon() {
        let tray = TrayIconBuilder::new()
            .icon(icon.clone())
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| match event.id.as_ref() {
                "quit" => app.exit(0),
                _ => {}
            })
            .build(app)?;
        // Keep the tray alive for the entire app lifetime
        std::mem::forget(tray);
    }
    Ok(())
}

#[cfg(desktop)]
fn register_global_shortcut(app: &App) -> tauri::Result<()> {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    let shortcut = Shortcut::new(Some(Modifiers::ALT | Modifiers::SUPER), Code::Space);
    let listen_shortcut = shortcut.clone();

    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |app, current_shortcut, event| {
                if current_shortcut == &listen_shortcut && event.state() == ShortcutState::Pressed {
                    handle_hotkey_pressed(app);
                }
            })
            .build(),
    )?;

    app.global_shortcut().register(shortcut).map_err(|error| {
        tauri::Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            error.to_string(),
        ))
    })?;
    Ok(())
}

#[cfg(not(desktop))]
fn register_global_shortcut(_app: &App) -> tauri::Result<()> {
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(HotkeyState::default())
        .plugin(tauri_plugin_opener::init())
        // A2: Launch at login
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            register_global_shortcut(app)?;

            // A3: System tray
            if let Err(e) = setup_tray(app) {
                eprintln!("tray setup failed: {e}");
            }

            // A2: Enable autostart on first run
            #[cfg(desktop)]
            {
                use tauri_plugin_autostart::ManagerExt;
                let autolaunch = app.autolaunch();
                if !autolaunch.is_enabled().unwrap_or(false) {
                    let _ = autolaunch.enable();
                }
            }

            if let Some(window) = app.get_webview_window("main") {
                place_window_to_top_left(&window);
                let _ = window.set_ignore_cursor_events(true);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_click_through,
            read_text,
            write_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
