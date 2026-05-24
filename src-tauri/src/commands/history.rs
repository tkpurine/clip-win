/// 履歴関連の Tauri コマンド
/// フロントエンド（設定・スニペット管理画面）から invoke() で呼ばれる
use tauri::State;

use crate::{
    core::tray::rebuild_tray,
    models::clipboard_entry::ClipboardEntry,
    AppState,
};

#[tauri::command]
pub fn get_history(state: State<'_, AppState>, limit: Option<i64>) -> Vec<ClipboardEntry> {
    let limit = limit.unwrap_or(100);
    state.storage.get_history(limit)
}

#[tauri::command]
pub fn delete_entry(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    id: i64,
) -> Result<(), String> {
    state
        .storage
        .delete_history(id)
        .map_err(|e| e.to_string())?;
    rebuild_tray(&app);
    Ok(())
}

#[tauri::command]
pub fn toggle_pin(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    id: i64,
) -> Result<(), String> {
    state
        .storage
        .toggle_pin(id)
        .map_err(|e| e.to_string())?;
    rebuild_tray(&app);
    Ok(())
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    state
        .storage
        .clear_history()
        .map_err(|e| e.to_string())?;
    rebuild_tray(&app);
    Ok(())
}
