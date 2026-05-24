// Windows でリリースビルド時にコンソールウィンドウを非表示にする（必須）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    clip_win_lib::run()
}
