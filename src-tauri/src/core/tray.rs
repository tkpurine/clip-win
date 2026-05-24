/// トレイアイコン・メニュー構築モジュール
///
/// # MenuId 命名規則
/// ```
/// hist_paste_{id}      履歴アイテム（クリックで即ペースト）
/// hist_sub_{id}_paste  サブメニュー: 貼り付け
/// hist_sub_{id}_pin    サブメニュー: ピン留め / 解除
/// hist_sub_{id}_del    サブメニュー: 削除
/// pin_paste_{id}       ピン留めアイテム（クリックで即ペースト）
/// clear_history        履歴をすべて削除
/// open_settings        設定画面を開く
/// quit                 終了
/// ```
use tauri::{
    image::Image,
    menu::{Menu, MenuBuilder, MenuItem, Submenu, SubmenuBuilder},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

use crate::models::clipboard_entry::ClipboardEntry;

/// Phase 1 で表示する最大文字数
const ITEM_DISPLAY_LEN: usize = 40;

// ============================================================================
// トレイアイコン構築
// ============================================================================

/// トレイアイコンを初期化する（アプリ起動時に 1 度呼ぶ）。
pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    // アイコン画像を埋め込み読み込み
    let icon = Image::from_bytes(include_bytes!("../../icons/icon.png"))
        .expect("トレイアイコンの読み込みに失敗しました");

    // 初期メニューを構築
    let state = app.state::<crate::AppState>();
    let display_count = state
        .storage
        .get_setting("menu_display_count")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(10);
    let history = state.storage.get_history(display_count + 50); // ピン留め分を余分に取得
    let menu = build_menu(app, &history)?;

    // メニューを AppState に保存（ホットキー時の popup に使う）
    *state.tray_menu.lock().unwrap() = Some(menu.clone());

    // トレイアイコンを作成
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("clip-win")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id().as_ref());
        })
        .build(app)?;

    Ok(())
}

// ============================================================================
// メニュー構築
// ============================================================================

/// 現在の履歴・スニペットからネイティブメニューを構築する。
pub fn build_menu(app: &AppHandle, history: &[ClipboardEntry]) -> tauri::Result<Menu<tauri::Wry>> {
    let pinned: Vec<&ClipboardEntry> = history.iter().filter(|e| e.is_pinned).collect();
    let unpinned: Vec<&ClipboardEntry> = history.iter().filter(|e| !e.is_pinned).collect();

    let state = app.state::<crate::AppState>();
    let display_count = state
        .storage
        .get_setting("menu_display_count")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(10) as usize;

    let mut builder = MenuBuilder::new(app);

    // ── ピン留めアイテム ──────────────────────────────────────────────
    for entry in &pinned {
        let title = format!("📌 {}", entry.display_title(ITEM_DISPLAY_LEN));
        let submenu = build_item_submenu(app, entry, true)?;
        builder = builder.item(&submenu);
    }

    if !pinned.is_empty() {
        builder = builder.separator();
    }

    // ── 通常履歴アイテム（最大 display_count 件）────────────────────
    let visible = unpinned.iter().take(display_count);
    for entry in visible {
        let submenu = build_item_submenu(app, entry, false)?;
        builder = builder.item(&submenu);
    }

    // 件数が display_count を超える場合「すべて表示」サブメニューを追加
    if unpinned.len() > display_count {
        builder = builder.separator();
        let all_menu = build_all_history_submenu(app, &unpinned)?;
        builder = builder.item(&all_menu);
    }

    builder = builder.separator();

    // ── 履歴削除 ─────────────────────────────────────────────────────
    builder = builder.item(&MenuItem::with_id(
        app,
        "clear_history",
        "履歴をすべて削除",
        true,
        None::<&str>,
    )?);

    builder = builder.separator();

    // ── 設定・終了 ───────────────────────────────────────────────────
    builder = builder.item(&MenuItem::with_id(
        app,
        "open_settings",
        "設定...",
        true,
        None::<&str>,
    )?);
    builder = builder.item(&MenuItem::with_id(
        app,
        "quit",
        "終了",
        true,
        None::<&str>,
    )?);

    builder.build()
}

/// 履歴アイテム 1 件分のサブメニューを構築する。
/// ▶ ホバーで「貼り付け / ピン留め / 削除」を表示。
fn build_item_submenu(
    app: &AppHandle,
    entry: &ClipboardEntry,
    is_pinned: bool,
) -> tauri::Result<Submenu<tauri::Wry>> {
    let prefix = if is_pinned { "pin" } else { "hist" };
    let title = entry.display_title(ITEM_DISPLAY_LEN);

    let paste_item = MenuItem::with_id(
        app,
        format!("{}_sub_{}_paste", prefix, entry.id),
        "📋 貼り付け",
        true,
        None::<&str>,
    )?;
    let pin_label = if is_pinned {
        "📌 ピン留め解除"
    } else {
        "📌 ピン留め"
    };
    let pin_item = MenuItem::with_id(
        app,
        format!("{}_sub_{}_pin", prefix, entry.id),
        pin_label,
        true,
        None::<&str>,
    )?;
    let del_item = MenuItem::with_id(
        app,
        format!("{}_sub_{}_del", prefix, entry.id),
        "🗑 削除",
        true,
        None::<&str>,
    )?;

    SubmenuBuilder::new(app, title)
        .item(&paste_item)
        .item(&pin_item)
        .item(&del_item)
        .build()
}

/// 「すべての履歴」サブメニューを構築する。
fn build_all_history_submenu(
    app: &AppHandle,
    unpinned: &[&ClipboardEntry],
) -> tauri::Result<Submenu<tauri::Wry>> {
    let mut builder = SubmenuBuilder::new(app, "すべての履歴を表示");
    for entry in unpinned {
        let submenu = build_item_submenu(app, entry, false)?;
        builder = builder.item(&submenu);
    }
    builder.build()
}

// ============================================================================
// メニュー再構築（クリップボード変更時などに呼ぶ）
// ============================================================================

/// 最新の履歴でトレイメニューを再構築する。
/// メインスレッドで実行するため `run_on_main_thread` を使用する。
pub fn rebuild_tray(app: &AppHandle) {
    let app_clone = app.clone();
    let _ = app.run_on_main_thread(move || {
        let state = app_clone.state::<crate::AppState>();
        let display_count = state
            .storage
            .get_setting("menu_display_count")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(10);

        let history = state.storage.get_history(display_count + 50);

        match build_menu(&app_clone, &history) {
            Ok(new_menu) => {
                // AppState に保存
                *state.tray_menu.lock().unwrap() = Some(new_menu.clone());
                // トレイアイコンのメニューを更新
                if let Some(tray) = app_clone.tray_by_id("main") {
                    if let Err(e) = tray.set_menu(Some(&new_menu)) {
                        log::warn!("トレイメニューの更新に失敗しました: {}", e);
                    }
                }
            }
            Err(e) => {
                log::warn!("メニューの構築に失敗しました: {}", e);
            }
        }
    });
}

// ============================================================================
// メニューイベント処理
// ============================================================================

/// メニューイベントを処理する。
/// `on_menu_event` ハンドラから呼ぶ。
pub fn handle_menu_event(app: &AppHandle, id: &str) {
    log::debug!("メニューイベント: {}", id);

    if id == "quit" {
        app.exit(0);
        return;
    }

    if id == "clear_history" {
        let state = app.state::<crate::AppState>();
        if let Err(e) = state.storage.clear_history() {
            log::warn!("履歴の削除に失敗しました: {}", e);
        }
        rebuild_tray(app);
        return;
    }

    if id == "open_settings" {
        if let Some(win) = app.get_webview_window("settings") {
            let _ = win.show();
            let _ = win.set_focus();
        }
        return;
    }

    // hist_sub_{id}_paste / hist_sub_{id}_pin / hist_sub_{id}_del
    if let Some(rest) = id.strip_prefix("hist_sub_").or_else(|| id.strip_prefix("pin_sub_")) {
        // rest = "42_paste" / "42_pin" / "42_del"
        if let Some((id_str, action)) = rest.split_once('_') {
            if let Ok(entry_id) = id_str.parse::<i64>() {
                handle_entry_action(app, entry_id, action);
            }
        }
    }
}

/// 履歴アイテムへのアクション（貼り付け / ピン留め / 削除）を処理する。
fn handle_entry_action(app: &AppHandle, entry_id: i64, action: &str) {
    let state = app.state::<crate::AppState>();

    match action {
        "paste" => {
            // 対象エントリのテキストを取得してペースト
            let history = state.storage.get_history(200);
            if let Some(entry) = history.into_iter().find(|e| e.id == entry_id) {
                let text = entry.content.clone();
                let is_writing = state.is_writing.clone();
                let prev_hwnd = *state.prev_hwnd.lock().unwrap();
                let app_clone = app.clone();

                tauri::async_runtime::spawn(async move {
                    crate::core::paste::paste_to(prev_hwnd, &text, is_writing).await;
                    // ペースト後にメニューを更新（必要であれば）
                    // rebuild_tray(&app_clone);
                });
            }
        }
        "pin" => {
            if let Err(e) = state.storage.toggle_pin(entry_id) {
                log::warn!("ピン留めの切り替えに失敗しました: {}", e);
            }
            rebuild_tray(app);
        }
        "del" => {
            if let Err(e) = state.storage.delete_history(entry_id) {
                log::warn!("履歴の削除に失敗しました: {}", e);
            }
            rebuild_tray(app);
        }
        _ => {
            log::warn!("不明なアクション: {}", action);
        }
    }
}
