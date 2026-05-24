# clip-win

Windows 向け軽量クリップボードマネージャー。  
macOS の [Clipy](https://github.com/Clipy/Clipy) にインスパイアされ、  
**テキスト履歴 + スニペット** をネイティブメニューで即呼び出し。

---

## 特徴

- 🚀 **軽量常駐** — トレイアイコンのみで WebView を起動しないため常駐メモリ 10〜15 MB
- ⌨️ **ホットキー呼び出し** — `Ctrl+Shift+V` でクリップボード履歴をネイティブメニュー表示
- 📌 **ピン留め** — よく使う履歴アイテムをピン留めして削除から保護
- 🗂 **スニペット**（Phase 2 予定） — 定型文をフォルダ分類して即ペースト
- ⚙️ **設定画面**（Phase 3 予定） — ホットキー変更・履歴上限・自動起動

---

## ドキュメント

- [設計書](docs/design.md)

---

## 動作要件

| 項目 | 要件 |
|------|------|
| OS | Windows 10 (1903 以降) または Windows 11 |
| WebView2 | Windows 10/11 に標準搭載済み（追加インストール不要） |
| 管理者権限 | 不要 |

> **注意:** 本アプリは **Windows 専用** です。`cargo tauri dev` での動作確認も Windows 環境が必要です。

---

## 開発環境セットアップ

### 必要なツール

| ツール | バージョン | 用途 |
|--------|-----------|------|
| Rust | 1.77 以上 (stable) | バックエンド |
| Node.js | 18 以上 LTS | フロントエンドビルド・Tauri CLI |
| VS Build Tools | 2022 | Rust の Windows ターゲットビルドに必要 |
| Git | 最新 | ソース管理 |

### 方法 A: 自動セットアップ（推奨）

PowerShell を **管理者権限** で開き、以下を実行します。

```powershell
# リポジトリをクローン
git clone https://github.com/tkpurine/clip-win.git
cd clip-win

# 自動セットアップスクリプトを実行
.\scripts\install.ps1
```

スクリプトが以下をすべて自動インストールします：

- Visual Studio Build Tools 2022（C++ デスクトップ開発ワークロード）
- Rust (rustup)
- Node.js LTS
- Tauri CLI v2

### 方法 B: 手動セットアップ

#### Step 1: Visual Studio Build Tools

Rust のコンパイルに必要な C++ ビルドツールを先にインストールします。

```powershell
winget install Microsoft.VisualStudio.2022.BuildTools `
  --override "--quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

または https://visualstudio.microsoft.com/visual-cpp-build-tools/ からダウンロードして  
**「C++ によるデスクトップ開発」** を選択してインストール。

#### Step 2: Rust

```powershell
winget install Rustlang.Rustup

# PowerShell を再起動してから確認
rustc --version   # rustc 1.77.x 以上
cargo --version
```

#### Step 3: Node.js

```powershell
winget install OpenJS.NodeJS.LTS

# 確認
node --version   # v18.x 以上
npm --version
```

#### Step 4: Tauri CLI

```powershell
cargo install tauri-cli --version "^2.0"

# 確認
cargo tauri --version   # tauri-cli 2.x
```

---

## ビルドと起動

```powershell
# リポジトリをクローン（まだの場合）
git clone https://github.com/tkpurine/clip-win.git
cd clip-win

# フロントエンド依存パッケージをインストール
npm install

# 開発モードで起動（初回は Rust クレートのコンパイルで 5〜10 分かかる場合あり）
cargo tauri dev
```

起動すると：

1. システムトレイにアイコンが表示される
2. `Ctrl+Shift+V` でクリップボード履歴メニューが開く
3. トレイアイコンを右クリックしてもメニューが開く

### リリースビルド

```powershell
# インストーラーを生成
cargo tauri build

# 生成物の場所
# src-tauri/target/release/bundle/nsis/clip-win_x.x.x_x64-setup.exe
# src-tauri/target/release/bundle/msi/clip-win_x.x.x_x64_en-US.msi
```

---

## 推奨 VS Code 拡張機能

| 拡張機能 ID | 用途 |
|------------|------|
| `rust-lang.rust-analyzer` | Rust 補完・型チェック |
| `tauri-apps.tauri-vscode` | Tauri サポート |
| `svelte.svelte-vscode` | Svelte 補完 |
| `esbenp.prettier-vscode` | フロントエンド整形 |

---

## ステータス

| フェーズ | 内容 | 状態 |
|----------|------|------|
| Phase 1 | コア機能（履歴監視・ホットキー・ペースト） | ✅ 実装完了 |
| Phase 2 | スニペット機能 | 🔜 未着手 |
| Phase 3 | 設定画面・品質改善 | 🔜 未着手 |
| Phase 4 | 配布準備・リリース | 🔜 未着手 |
