use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

pub async fn paste_text(app: &AppHandle, text: &str) -> Result<(), String> {
    app.clipboard()
        .write_text(text.to_string())
        .map_err(|e| e.to_string())?;

    // Wait for focus to return to the original app after overlay is hidden
    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
    simulate_paste();

    Ok(())
}

pub async fn replace_line_text(
    app: &AppHandle,
    text: &str,
    clear_passes: usize,
) -> Result<(), String> {
    app.clipboard()
        .write_text(text.to_string())
        .map_err(|e| e.to_string())?;

    // Terminal selections are not editable ranges. Clear the whole prompt line
    // first, then paste the sharpened text into Claude/Codex-style inputs.
    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
    simulate_clear_line(clear_passes);
    tokio::time::sleep(tokio::time::Duration::from_millis(40)).await;
    simulate_paste();

    Ok(())
}

fn simulate_clear_line(clear_passes: usize) {
    if let Ok(mut e) = Enigo::new(&Settings::default()) {
        for _ in 0..clear_passes {
            let _ = e.key(Key::Control, Press);
            let _ = e.key(Key::Unicode('u'), Click);
            let _ = e.key(Key::Control, Release);
            let _ = e.key(Key::Backspace, Click);
            std::thread::sleep(std::time::Duration::from_millis(8));
        }

    }
}

fn simulate_paste() {
    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    if let Ok(mut e) = Enigo::new(&Settings::default()) {
        let _ = e.key(modifier, Press);
        let _ = e.key(Key::Unicode('v'), Click);
        let _ = e.key(modifier, Release);
    }
}
