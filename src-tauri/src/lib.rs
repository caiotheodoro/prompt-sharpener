use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

mod clipboard;
mod hotkey;
mod llm;
mod settings;
mod window;

#[tauri::command]
async fn sharpen_prompt(
    text: String,
    mode: String,
    provider: String,
    api_key: String,
    model: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let system_prompt = settings::get(&app, "system_prompt").ok();
    llm::call(&text, &mode, system_prompt, &provider, &api_key, &model).await
}

#[tauri::command]
async fn get_setting(key: String, app: tauri::AppHandle) -> Result<String, String> {
    settings::get(&app, &key)
}

#[tauri::command]
async fn set_setting(key: String, value: String, app: tauri::AppHandle) -> Result<(), String> {
    settings::set(&app, &key, &value)
}

#[tauri::command]
async fn get_default_system_prompt() -> Result<String, String> {
    Ok(llm::default_system_prompt())
}

#[tauri::command]
async fn update_hotkey(shortcut: String, app: tauri::AppHandle) -> Result<(), String> {
    hotkey::update(&app, &shortcut)
}

#[tauri::command]
async fn paste_text(text: String, app: tauri::AppHandle) -> Result<(), String> {
    clipboard::paste_text(&app, &text).await
}

#[tauri::command]
async fn replace_line_text(
    text: String,
    clear_passes: usize,
    app: tauri::AppHandle,
) -> Result<(), String> {
    clipboard::replace_line_text(&app, &text, clear_passes).await
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    window::show_settings(&app);
}

#[tauri::command]
async fn check_ollama_status() -> Result<(), String> {
    llm::check_ollama_status().await
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            window::install_settings_close_handler(app);

            let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;

            let icon = app
                .default_window_icon()
                .cloned()
                .expect("no app icon configured");

            TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "settings" => window::show_settings(app),
                    "quit" => std::process::exit(0),
                    _ => {}
                })
                .build(app)?;

            hotkey::register(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            sharpen_prompt,
            get_setting,
            set_setting,
            get_default_system_prompt,
            update_hotkey,
            paste_text,
            replace_line_text,
            open_settings,
            check_ollama_status,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
