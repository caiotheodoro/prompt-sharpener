use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

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
    #[cfg(target_os = "macos")]
    let modifier = Modifiers::SUPER | Modifiers::ALT;
    #[cfg(not(target_os = "macos"))]
    let modifier = Modifiers::CONTROL | Modifiers::ALT;

    let shortcut = Shortcut::new(Some(modifier), Code::KeyP);
    let handle = app.handle().clone();

    match app
        .global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                trigger_capture(&handle);
            }
        }) {
        Ok(_) => eprintln!("[hotkey] Ctrl+Alt+P registered"),
        Err(e) => eprintln!("[hotkey] FAILED to register: {e}"),
    }

    Ok(())
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
