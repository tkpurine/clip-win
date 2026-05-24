/// 自動ペーストモジュール
///
/// # 動作フロー
/// 1. `is_writing = true` に設定（ClipboardWatcher が自己検知しないようにする）
/// 2. `arboard` でクリップボードにテキストをセット
/// 3. 元ウィンドウにフォーカスを戻す（Win32: `SetForegroundWindow`）
/// 4. `enigo` で Ctrl+V を送信
/// 5. 少し待ってから `is_writing = false` に戻す
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use tokio::time::sleep;

// ============================================================================
// 前ウィンドウ記憶
// ============================================================================

/// 現在のフォアグラウンドウィンドウのハンドル（isize）を取得する。
///
/// # 重要
/// ホットキー / トレイクリック処理の **前** に呼ぶこと。
/// 呼ぶタイミングが遅れると clip-win 自身が「前のウィンドウ」になってしまう。
#[cfg(target_os = "windows")]
pub fn capture_foreground() -> isize {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    unsafe { GetForegroundWindow().0 }
}

#[cfg(not(target_os = "windows"))]
pub fn capture_foreground() -> isize {
    0
}

// ============================================================================
// 自動ペースト（非同期）
// ============================================================================

/// クリップボードにテキストをセットし、元ウィンドウに Ctrl+V を送信する。
///
/// # 引数
/// - `hwnd`: 元のフォアグラウンドウィンドウのハンドル（`capture_foreground()` の戻り値）
/// - `text`: ペーストするテキスト
/// - `is_writing`: ClipboardWatcher の自己検知抑制フラグ
pub async fn paste_to(
    #[cfg_attr(not(target_os = "windows"), allow(unused_variables))]
    hwnd: isize,
    text: &str,
    is_writing: Arc<AtomicBool>,
) {
    // 1. 書き込みフラグを立てる（ClipboardWatcher が無視するようにする）
    is_writing.store(true, Ordering::SeqCst);

    // 2. arboard でクリップボードにテキストをセット
    let text_owned = text.to_string();
    let write_ok = tokio::task::spawn_blocking(move || {
        match arboard::Clipboard::new() {
            Ok(mut cb) => cb.set_text(&text_owned).is_ok(),
            Err(e) => {
                log::error!("クリップボードの初期化に失敗しました: {}", e);
                false
            }
        }
    })
    .await
    .unwrap_or(false);

    if !write_ok {
        log::warn!("クリップボードへの書き込みに失敗しました");
        is_writing.store(false, Ordering::SeqCst);
        return;
    }

    // 3. 元のウィンドウにフォーカスを戻す（Windows 専用）
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::SetForegroundWindow};
        if hwnd != 0 {
            unsafe {
                let _ = SetForegroundWindow(HWND(hwnd));
            }
        }
    }

    // フォーカス移動を待つ
    sleep(Duration::from_millis(50)).await;

    // 4. Ctrl+V を送信（enigo 使用）
    send_ctrl_v().await;

    // 5. WM_CLIPBOARDUPDATE の処理完了を待ってからフラグを戻す
    sleep(Duration::from_millis(150)).await;
    is_writing.store(false, Ordering::SeqCst);
}

/// `enigo` を使って Ctrl+V を送信する。
/// `spawn_blocking` で同期処理をブロッキングスレッドで実行する。
async fn send_ctrl_v() {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let result = tokio::task::spawn_blocking(|| {
        let mut enigo = match Enigo::new(&Settings::default()) {
            Ok(e) => e,
            Err(err) => {
                log::error!("Enigo の初期化に失敗: {}", err);
                return;
            }
        };
        let _ = enigo.key(Key::Control, Direction::Press);
        let _ = enigo.key(Key::Unicode('v'), Direction::Click);
        let _ = enigo.key(Key::Control, Direction::Release);
    })
    .await;

    if let Err(e) = result {
        log::error!("Ctrl+V の送信に失敗しました: {}", e);
    }
}
