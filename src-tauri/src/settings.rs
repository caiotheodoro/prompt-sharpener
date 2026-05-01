use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE: &str = "settings.json";

pub fn get(app: &AppHandle, key: &str) -> Result<String, String> {
    let store = app.store(STORE).map_err(|e| e.to_string())?;
    store
        .get(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .ok_or_else(|| format!("Key '{key}' not found"))
}

pub fn set(app: &AppHandle, key: &str, value: &str) -> Result<(), String> {
    let store = app.store(STORE).map_err(|e| e.to_string())?;
    store.set(key, serde_json::Value::String(value.to_string()));
    store.save().map_err(|e| e.to_string())
}
