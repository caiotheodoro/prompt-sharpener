use tauri::{App, AppHandle, Manager, PhysicalPosition, WindowEvent};

pub fn show_overlay(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("overlay") {
        let (cx, cy) = cursor_position(&win);
        let win_w = 640i32;
        let max_h = 420i32; // budget for vertical clamping; JS sets final height

        if let Ok(Some(monitor)) = win.current_monitor() {
            let screen = monitor.size();
            // Offset right and below cursor so the selected text remains visible
            let x = (cx + 20).min(screen.width as i32 - win_w).max(0);
            // Place below cursor if room; otherwise above
            let y = if cy + 20 + max_h < screen.height as i32 {
                cy + 20
            } else {
                (cy - max_h - 10).max(0)
            };
            win.set_position(PhysicalPosition::new(x, y)).ok();
        }

        win.show().ok();
        win.set_focus().ok();
    }
}

pub fn install_settings_close_handler(app: &App) {
    if let Some(win) = app.get_webview_window("settings") {
        let settings_win = win.clone();
        win.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                settings_win.hide().ok();
            }
        });
    }
}

pub fn show_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        win.show().ok();
        win.set_focus().ok();
    }
}

fn cursor_position(win: &tauri::WebviewWindow) -> (i32, i32) {
    if let Ok(pos) = win.cursor_position() {
        return (pos.x as i32, pos.y as i32);
    }
    #[cfg(target_os = "linux")]
    if let Ok(out) = std::process::Command::new("xdotool")
        .args(["getmouselocation", "--shell"])
        .output()
    {
        if let Ok(text) = String::from_utf8(out.stdout) {
            let mut x = 0i32;
            let mut y = 0i32;
            for line in text.lines() {
                if let Some(v) = line.strip_prefix("X=") {
                    x = v.parse().unwrap_or(0);
                }
                if let Some(v) = line.strip_prefix("Y=") {
                    y = v.parse().unwrap_or(0);
                }
            }
            return (x, y);
        }
    }
    (0, 0)
}
