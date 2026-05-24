/// clip-win — Tauri v2 エントリポイント
///
/// AppState の定義と Tauri アプリのセットアップを担う。
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

pub mod commands;
pub mod core;
pub mod models;

// ============================================================================
// AppState
// ============================================================================

/// アプリケーション全体で共有する状態。
/// `tauri::Builder::manage()` で登録し、各ハンドラから `State<AppState>` で参照する。
pub struct AppState {
    /// SQLite CRUD サービス（内部に `Mutex<Connection>` を持つ）
    pub storage: Arc<crate::core::storage::StorageService>,

    /// ペースト処理中フラグ（ClipboardWatcher の自己検知を防ぐ）
    /// `Mutex<bool>` ではなく `AtomicBool` を使う理由:
    /// クリップボード監視スレッドからロック待ちするとイベント取りこぼしが起きるため。
    pub is_writing: Arc<AtomicBool>,

    /// ホットキー押下直前のフォアグラウンドウィンドウハンドル（isize で保持）
    pub prev_hwnd: Mutex<isize>,

    /// 現在のトレイメニュー（ホットキーによる popup 表示に使う）
    pub tray_menu: Mutex<Option<tauri::menu::Menu<tauri::Wry>>>,
}

// ============================================================================
// Tauri アプリ実行
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            setup_app(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::history::get_history,
            commands::history::delete_entry,
            commands::history::toggle_pin,
            commands::history::clear_history,
            commands::settings::get_settings,
            commands::settings::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("clip-win の起動に失敗しました");
}

// ============================================================================
// セットアップ処理
// ============================================================================

fn setup_app(app: &mut tauri::App) -> tauri::Result<()> {
    // ── 1. DB パスを取得して StorageService を初期化 ─────────────────
    let db_dir = app
        .path()
        .app_data_dir()
        .expect("AppData ディレクトリの取得に失敗しました");
    std::fs::create_dir_all(&db_dir)
        .expect("AppData ディレクトリの作成に失敗しました");
    let db_path = db_dir.join("clip-win.db");
    let db_path_str = db_path
        .to_str()
        .expect("DB パスの変換に失敗しました");

    let storage = Arc::new(
        crate::core::storage::StorageService::new(db_path_str)
            .expect("StorageService の初期化に失敗しました"),
    );
    let is_writing = Arc::new(AtomicBool::new(false));

    // ── 2. AppState を管理に登録 ──────────────────────────────────────
    app.manage(AppState {
        storage: storage.clone(),
        is_writing: is_writing.clone(),
        prev_hwnd: Mutex::new(0isize),
        tray_menu: Mutex::new(None),
    });

    // ── 3. トレイアイコンを構築 ───────────────────────────────────────
    crate::core::tray::build_tray(app.handle())?;

    // ── 4. クリップボード監視を開始 ───────────────────────────────────
    crate::core::clipboard::start_clipboard_watcher(
        storage,
        is_writing,
        app.handle().clone(),
    );

    // ── 5. グローバルホットキーを登録 ─────────────────────────────────
    let hotkey_str = app
        .state::<AppState>()
        .storage
        .get_setting("hotkey")
        .unwrap_or_else(|| "Ctrl+Shift+V".to_string());

    app.handle()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcut(hotkey_str.as_str())?
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        on_hotkey_pressed(app);
                    }
                })
                .build(),
        )?;

    log::info!("clip-win の起動が完了しました");
    Ok(())
}

// ============================================================================
// ホットキー処理
// ============================================================================

/// ホットキー（デフォルト: Ctrl+Shift+V）が押されたときの処理。
///
/// ## 重要な順序
/// 1. `capture_foreground()` を **ウィンドウ表示前** に呼ぶ
///    → 表示後に呼ぶと clip-win 自身が「前のウィンドウ」になってしまう
/// 2. hotkey-trigger ウィンドウを表示してフォーカスを確保
/// 3. 最新の履歴でメニューを再構築
/// 4. メニューを popup で表示（blocking なので別スレッドで実行）
fn on_hotkey_pressed(app: &AppHandle) {
    // 1. 前のフォアグラウンドウィンドウを記憶
    let prev_hwnd = crate::core::paste::capture_foreground();
    {
        let state = app.state::<AppState>();
        *state.prev_hwnd.lock().unwrap() = prev_hwnd;
    }

    let app_clone = app.clone();
    std::thread::spawn(move || {
        // 2. hotkey-trigger ウィンドウを表示してフォーカスを確保
        let trigger_win = app_clone.get_webview_window("hotkey-trigger");
        if let Some(win) = &trigger_win {
            let _ = win.show();
            let _ = win.set_focus();
        }

        // 3. 最新の履歴でメニューを再構築
        crate::core::tray::rebuild_tray(&app_clone);

        // 4. メニューを popup で表示（Windows では blocking）
        let state = app_clone.state::<AppState>();
        let menu_guard = state.tray_menu.lock().unwrap();
        if let Some(menu) = menu_guard.as_ref() {
            if let Some(win) = &trigger_win {
                if let Err(e) = menu.popup(win.clone()) {
                    log::warn!("メニューの表示に失敗しました: {}", e);
                }
            }
        }
        drop(menu_guard);

        // 5. popup が閉じた後（popup は blocking）、ウィンドウを再非表示にする
        if let Some(win) = &trigger_win {
            let _ = win.hide();
        }
    });
}
