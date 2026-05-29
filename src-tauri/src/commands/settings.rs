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
    // 1. ホワイトリストチェック: 未知のキーはエラー
    const ALLOWED_KEYS: &[&str] = &[
        "history_limit",
        "menu_display_count",
        "hotkey",
        "language",
        "startup_enabled",
    ];
    for key in settings.keys() {
        if !ALLOWED_KEYS.contains(&key.as_str()) {
            return Err(format!("不明な設定キー: {}", key));
        }
    }

    // 2. ホットキーが含まれる場合は先に登録を試みる（DB 保存前の検証）
    // 登録に失敗した場合は旧ホットキーに戻し、DB も更新しない
    if let Some(new_hotkey) = settings.get("hotkey") {
        crate::re_register_hotkey(&app, new_hotkey.as_str())?;
    }

    // 3. DB に保存
    let mut db_error: Option<String> = None;
    for (key, value) in &settings {
        if let Err(e) = state.storage.set_setting(key, value) {
            db_error = Some(e.to_string());
            break;
        }
    }

    // 4. DB 保存失敗時: ホットキーが変更されていた場合は元に戻す
    if let Some(err) = db_error {
        if settings.contains_key("hotkey") {
            // DB の旧ホットキーで再登録（失敗しても続行）
            let old_hotkey = state
                .storage
                .get_setting("hotkey")
                .unwrap_or_else(|| "Ctrl+Shift+V".to_string());
            if let Err(e) = crate::re_register_hotkey(&app, &old_hotkey) {
                log::warn!("DB 保存失敗後のホットキーロールバックに失敗: {}", e);
            }
        }
        return Err(err);
    }

    Ok(())
}
