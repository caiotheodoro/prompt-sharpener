use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};
use serde::Serialize;
use std::str::FromStr;
use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CapturedText {
    text: String,
    accept_behavior: AcceptBehavior,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum AcceptBehavior {
    Paste,
    ReplaceLine,
}

pub fn register(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let configured_shortcut =
        crate::settings::get(app.handle(), "hotkey").unwrap_or_else(|_| default_shortcut_label());
    if let Err(err) = register_shortcut(app.handle(), &configured_shortcut) {
        eprintln!("[hotkey] configured shortcut failed, falling back to default: {err}");
        register_shortcut(app.handle(), &default_shortcut_label())?;
    }

    Ok(())
}

pub fn update(app: &AppHandle, shortcut_label: &str) -> Result<(), String> {
    let normalized = normalize_shortcut_label(shortcut_label)?;
    let previous = crate::settings::get(app, "hotkey").unwrap_or_else(|_| default_shortcut_label());

    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;
    if let Err(err) = register_shortcut(app, &normalized) {
        let _ = register_shortcut(app, &previous);
        return Err(err.to_string());
    }

    crate::settings::set(app, "hotkey", &normalized)?;
    Ok(())
}

pub fn default_shortcut_label() -> String {
    #[cfg(target_os = "macos")]
    {
        "Super+Alt+P".to_string()
    }

    #[cfg(not(target_os = "macos"))]
    {
        "Ctrl+Alt+P".to_string()
    }
}

fn register_shortcut(
    app: &AppHandle,
    shortcut_label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut = Shortcut::from_str(shortcut_label)?;
    let handle = app.clone();

    match app
        .global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                trigger_capture(&handle);
            }
        }) {
        Ok(_) => eprintln!("[hotkey] {shortcut_label} registered"),
        Err(e) => eprintln!("[hotkey] FAILED to register: {e}"),
    }

    Ok(())
}

fn normalize_shortcut_label(shortcut_label: &str) -> Result<String, String> {
    let normalized = shortcut_label.trim().replace(' ', "");
    if normalized.is_empty() {
        return Err("Hotkey cannot be empty".to_string());
    }

    Shortcut::from_str(&normalized).map_err(|e| format!("Invalid hotkey: {e}"))?;
    Ok(normalized)
}

fn trigger_capture(app: &AppHandle) {
    simulate_copy();

    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));

        if let Ok(text) = app.clipboard().read_text() {
            let text = text.trim().to_string();
            if !text.is_empty() {
                let accept_mode = crate::settings::get(&app, "accept_mode")
                    .unwrap_or_else(|_| "terminal".to_string());
                let accept_behavior = active_accept_behavior(&accept_mode);
                app.emit(
                    "selected-text-captured",
                    CapturedText {
                        text,
                        accept_behavior,
                    },
                )
                .ok();
                crate::window::show_overlay(&app);
            }
        }
    });
}

fn simulate_copy() {
    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    if let Ok(mut e) = Enigo::new(&Settings::default()) {
        // The hotkey (Ctrl+Alt+P) may still be physically held when this fires.
        // Release those modifiers first so we send plain Ctrl+C, not Ctrl+Alt+C.
        let _ = e.key(Key::Alt, Release);
        let _ = e.key(modifier, Release);
        std::thread::sleep(std::time::Duration::from_millis(30));
        let _ = e.key(modifier, Press);
        let _ = e.key(Key::Unicode('c'), Click);
        let _ = e.key(modifier, Release);
    }
}

fn active_accept_behavior(accept_mode: &str) -> AcceptBehavior {
    match accept_mode {
        "editor" => AcceptBehavior::Paste,
        _ => AcceptBehavior::ReplaceLine,
    }
}
