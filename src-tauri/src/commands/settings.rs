/// 設定関連の Tauri コマンド（Phase 3 で本格利用予定）
use std::collections::HashMap;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> HashMap<String, String> {
    let keys = [
        "history_limit",
        "menu_display_count",
        "hotkey",
        "language",
        "startup_enabled",
    ];
    keys.iter()
        .filter_map(|&k| {
            state
                .storage
                .get_setting(k)
                .map(|v| (k.to_string(), v))
        })
        .collect()
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    settings: HashMap<String, String>,
) -> Result<(), String> {
    // ホットキーが含まれる場合は先に登録を試みる（DB 保存前の検証）
    // 登録に失敗した場合は旧ホットキーに戻し、DB も更新しない
    if let Some(new_hotkey) = settings.get("hotkey") {
        crate::re_register_hotkey(&app, new_hotkey.as_str())?;
    }

    // DB に保存
    for (key, value) in &settings {
        state
            .storage
            .set_setting(key, value)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
