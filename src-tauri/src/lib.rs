/// clip-win — Tauri v2 エントリポイント
///
/// AppState の定義と Tauri アプリのセットアップを担う。
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::ShortcutState;
use tauri_plugin_log::{Target, TargetKind};

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

    /// ホットキーメニュー表示中フラグ（多重起動防止）
    pub is_menu_open: Arc<AtomicBool>,
}

// ============================================================================
// Tauri アプリ実行
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // ログプラグイン: デバッグビルドはコンソール、リリースビルドはファイルに出力
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: Some("clip-win".into()) }),
                ])
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                })
                .build(),
        )
        // NOTE: global-shortcut プラグインは setup_app 内で .with_handler() 付きで
        //       一度だけ登録する。ここでの事前登録は二重登録エラーになるため不要。
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
        .build(tauri::generate_context!())
        .expect("clip-win の起動に失敗しました")
        .run(|_app, event| {
            // アプリ終了時に Win32 リソースを解放する
            if let tauri::RunEvent::Exit = event {
                crate::core::clipboard::stop_clipboard_watcher();
            }
        });
}

// ============================================================================
// ホットキー再登録
// ============================================================================

/// グローバルショートカットを新しいホットキーで再登録する。
///
/// 設定画面でホットキーが変更された際に呼ぶ。
/// 失敗した場合は旧ホットキーへの復元を試み、エラーメッセージを返す。
pub(crate) fn re_register_hotkey(app: &AppHandle, new_hotkey: &str) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    // 失敗時の復元用に現在の設定を読む
    let old_hotkey = app
        .state::<AppState>()
        .storage
        .get_setting("hotkey")
        .unwrap_or_else(|| "Ctrl+Shift+V".to_string());

    // 既存のショートカットをすべて解除
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;

    // 新しいホットキーを登録
    if let Err(e) = app.global_shortcut().register(new_hotkey) {
        log::warn!("新しいホットキー '{}' の登録に失敗: {}", new_hotkey, e);
        // 旧ホットキーに戻す（失敗しても続行）
        if let Err(e2) = app.global_shortcut().register(&old_hotkey) {
            log::error!("旧ホットキー '{}' への復元も失敗: {}", old_hotkey, e2);
        }
        return Err(format!("ホットキー '{}' は使用できません: {}", new_hotkey, e));
    }

    log::info!("ホットキーを {} に再登録しました", new_hotkey);
    Ok(())
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
        is_menu_open: Arc::new(AtomicBool::new(false)),
    });

    // ── 3. トレイアイコンを構築 ───────────────────────────────────────
    crate::core::tray::build_tray(app.handle())?;

    // ── 4. クリップボード監視を開始 ───────────────────────────────────
    crate::core::clipboard::start_clipboard_watcher(
        storage,
        is_writing,
        app.handle().clone(),
    );

    // ── 5. グローバルホットキーを登録（ここで1度だけ登録する）────────
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
/// 3. メニューをその場で構築（rebuild_tray に依存しない → 確実に最新データ）
/// 4. menu.popup() でメニューを表示（Windows では blocking）
/// 5. popup が閉じたら hotkey-trigger を非表示に戻す
fn on_hotkey_pressed(app: &AppHandle) {
    // 1. 前のフォアグラウンドウィンドウを記憶
    let prev_hwnd = crate::core::paste::capture_foreground();
    {
        let state = app.state::<AppState>();

        // 多重起動ガード: 既にメニューが開いている場合は何もしない
        if state
            .is_menu_open
            .swap(true, Ordering::SeqCst)
        {
            return;
        }

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

        // 3. 最新の履歴でメニューをその場で構築
        //    rebuild_tray() は run_on_main_thread で非同期なため、
        //    ここで直接構築することで確実に最新データを表示する。
        let state = app_clone.state::<AppState>();
        let history_limit = state
            .storage
            .get_setting("history_limit")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(100);
        let history = state.storage.get_history(history_limit);

        match crate::core::tray::build_menu(&app_clone, &history) {
            Ok(menu) => {
                if let Some(tray) = app_clone.tray_by_id("main") {
                    let _ = tray.set_menu(Some(&menu));
                }

                // 4. popup 表示（blocking: メニューが閉じるまで戻らない）
                if let Some(win) = &trigger_win {
                    if let Err(e) = menu.popup(win.clone()) {
                        log::warn!("メニューの表示に失敗しました: {}", e);
                    }
                }
            }
            Err(e) => {
                log::warn!("ホットキー用メニューの構築に失敗しました: {}", e);
            }
        }

        // 5. popup が閉じた後、ウィンドウを非表示に戻す
        if let Some(win) = &trigger_win {
            let _ = win.hide();
        }

        // 多重起動ガード解除
        state.is_menu_open.store(false, Ordering::SeqCst);
    });
}
