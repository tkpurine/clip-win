/// クリップボード監視モジュール
///
/// ## 設計
/// - **変更検知**: Windows 専用の `WM_CLIPBOARDUPDATE` メッセージを使う
///   （`AddClipboardFormatListener` でメッセージオンリーウィンドウに通知を受け取る）
/// - **テキスト読み取り**: `arboard` クレートを使用（Win32 型変換の複雑さを回避）
///
/// ## is_writing フラグ
/// 自アプリがクリップボードに書き込む際（ペースト処理中）、
/// ClipboardWatcher が自分自身の書き込みを履歴に追加しないよう `is_writing` フラグで抑制する。
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};

use tauri::AppHandle;

use crate::core::storage::StorageService;
use crate::core::tray::rebuild_tray;

// ============================================================================
// グローバル状態（Win32 ウィンドウプロシージャから参照するため static が必要）
// ============================================================================

/// クリップボード変更通知の送信チャネル（送信側）
static CLIPBOARD_TX: OnceLock<Mutex<std::sync::mpsc::Sender<()>>> = OnceLock::new();

/// ペースト処理中フラグへの参照
static IS_WRITING: OnceLock<Arc<AtomicBool>> = OnceLock::new();

// ============================================================================
// 公開 API
// ============================================================================

/// クリップボード監視を開始する。アプリ起動時に 1 度だけ呼ぶこと。
///
/// # 動作
/// 1. 受信スレッドを起動: 通知を受け取る → `arboard` で内容読み取り → StorageService → tray 再構築
/// 2. Windows: Win32 メッセージループスレッドを起動し `WM_CLIPBOARDUPDATE` を監視
pub fn start_clipboard_watcher(
    storage: Arc<StorageService>,
    is_writing: Arc<AtomicBool>,
    app_handle: AppHandle,
) {
    // グローバル参照を初期化（2 回目以降の呼び出しは無視される）
    IS_WRITING.get_or_init(|| is_writing);

    // チャネル: Win32 スレッド → 処理スレッド（単純な通知のみ送信）
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    CLIPBOARD_TX.get_or_init(|| Mutex::new(tx));

    // 受信スレッド: クリップボード変更を処理する
    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            // arboard でテキストを読み取る
            match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                Ok(text) => {
                    if let Err(e) = storage.add_history(&text) {
                        log::warn!("履歴の追加に失敗しました: {}", e);
                    }
                    rebuild_tray(&app_handle);
                }
                Err(_) => {
                    // テキスト以外のフォーマット（画像など）は無視
                }
            }
        }
        log::info!("クリップボード受信スレッドが終了しました");
    });

    // Windows: Win32 メッセージループスレッドを起動
    #[cfg(target_os = "windows")]
    start_win32_message_loop();
}

// ============================================================================
// Windows 固有実装: WM_CLIPBOARDUPDATE メッセージループ
// ============================================================================

/// Win32 メッセージループを別スレッドで起動する。
/// クリップボード変更（`WM_CLIPBOARDUPDATE`）を検知したら、
/// グローバルチャネルに通知を送る。
#[cfg(target_os = "windows")]
fn start_win32_message_loop() {
    std::thread::spawn(|| unsafe {
        use windows::{
            core::PCWSTR,
            Win32::{
                Foundation::HWND,
                System::{
                    DataExchange::AddClipboardFormatListener,
                    LibraryLoader::GetModuleHandleW,
                },
                UI::WindowsAndMessaging::{
                    CreateWindowExW, DispatchMessageW, GetMessageW, RegisterClassW, HWND_MESSAGE,
                    MSG, WINDOW_EX_STYLE, WNDCLASSW, WS_OVERLAPPED,
                },
            },
        };

        let instance = match GetModuleHandleW(PCWSTR::null()) {
            Ok(h) => h,
            Err(e) => {
                log::error!("GetModuleHandleW 失敗: {}", e);
                return;
            }
        };

        // ウィンドウクラスの登録
        let class_name: Vec<u16> = "ClipWinWatcher\0".encode_utf16().collect();
        let wnd_class = WNDCLASSW {
            hInstance: instance.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            lpfnWndProc: Some(wnd_proc),
            ..Default::default()
        };
        if RegisterClassW(&wnd_class) == 0 {
            log::error!(
                "RegisterClassW 失敗: {:?}",
                windows::core::Error::from_win32()
            );
            return;
        }

        // メッセージオンリーウィンドウを作成
        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            instance,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                log::error!("クリップボード監視ウィンドウの作成に失敗: {}", e);
                return;
            }
        };

        // クリップボード変更リスナーとして登録
        if let Err(e) = AddClipboardFormatListener(hwnd) {
            log::error!("AddClipboardFormatListener 失敗: {}", e);
            return;
        }

        log::info!("クリップボード監視を開始しました");

        // メッセージループ
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(0), 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
    });
}

/// Win32 ウィンドウプロシージャ（モジュールレベルで定義する必要がある）
///
/// `WM_CLIPBOARDUPDATE` を受信したら `CLIPBOARD_TX` に通知を送る。
/// テキストの実際の読み取りは受信スレッドで行う（`arboard` を使用）。
#[cfg(target_os = "windows")]
unsafe extern "system" fn wnd_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::{
        Foundation::LRESULT,
        UI::WindowsAndMessaging::{DefWindowProcW, WM_CLIPBOARDUPDATE},
    };

    if msg == WM_CLIPBOARDUPDATE {
        // 自アプリによる書き込み中は無視する
        let is_writing = IS_WRITING
            .get()
            .map(|f| f.load(Ordering::SeqCst))
            .unwrap_or(false);

        if !is_writing {
            if let Some(tx_lock) = CLIPBOARD_TX.get() {
                // try_lock: 取れなければ今回は無視（次の WM_CLIPBOARDUPDATE で拾われる）
                if let Ok(tx) = tx_lock.try_lock() {
                    let _ = tx.send(());
                }
            }
        }
        return LRESULT(0);
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}
