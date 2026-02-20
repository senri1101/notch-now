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

        let should_emit_emphasis = {
            let state = app_handle.state::<HotkeyState>();
            let mut inner = state.inner.lock().expect("hotkey state lock poisoned");
            if inner.sequence == token {
                inner.last_pressed_at = None;
                true
            } else {
                false
            }
        };

        if should_emit_emphasis {
            emit_mode(&app_handle, "emphasis");
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
        .setup(|app| {
            register_global_shortcut(app)?;

            if let Some(window) = app.get_webview_window("main") {
                place_window_to_top_left(&window);
                let _ = window.set_ignore_cursor_events(true);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![set_click_through])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
