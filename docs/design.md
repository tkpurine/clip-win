# clip-win 設計書

**バージョン**: 3.0
**作成日**: 2026-05-23
**更新日**: 2026-05-24
**ステータス**: Ready for Implementation

---

## 目次

1. [プロジェクト概要](#1-プロジェクト概要)
2. [要件定義](#2-要件定義)
3. [技術スタック選定](#3-技術スタック選定)
4. [システムアーキテクチャ](#4-システムアーキテクチャ)
5. [UI / UX 設計](#5-ui--ux-設計)
6. [データベース設計](#6-データベース設計)
7. [モジュール設計](#7-モジュール設計)
8. [開発フェーズ](#8-開発フェーズ)
9. [ビルド・配布方針](#9-ビルド配布方針)
10. [検討経緯（Decision Log）](#10-検討経緯decision-log)
11. [開発環境セットアップ](#11-開発環境セットアップ)
12. [次のアクション](#12-次のアクション)

---

## 1. プロジェクト概要

### 背景

macOS の [Clipy](https://github.com/Clipy/Clipy) は、  
「OS ネイティブのカスケードメニューで履歴・スニペットを即呼び出す」という  
シンプルかつ軽量な設計で高い評価を受けているクリップボードマネージャー。  
同等の体験を Windows 上で実現する。

Clipy 本体は **Swift + AppKit**（macOS ネイティブ）で実装されており、  
UI に `NSMenu`（OS 標準カスケードメニュー）をそのまま使っている。  
軽さの本質は「OS が提供するメニューを利用し、カスタム UI を一切持たない」点にある。

### ゴール

| # | ゴール |
|---|--------|
| G1 | コピー履歴をホットキー一発で呼び出してペーストできる |
| G2 | よく使う定型文をスニペットとして登録・即呼び出しできる |
| G3 | **軽量**（常駐メモリ 20 MB 以下、起動 1 秒以内） |
| G4 | 将来的に配布できる品質（単一バイナリ、インストーラー不要） |
| G5 | 将来的な macOS 対応を視野に入れたコード資産 |

### スコープ外（v1.0）

- 検索ボックス（シンプルさと軽量性を優先）
- 画像クリップボード対応
- クラウド同期・複数デバイス共有

---

## 2. 要件定義

### 2-1. 機能要件

#### クリップボード履歴

| ID | 要件 |
|----|------|
| F01 | テキストをコピーするたびに自動で履歴に記録する |
| F02 | 履歴はホットキー（デフォルト: `Ctrl+Shift+V`）で呼び出す |
| F03 | 履歴アイテムをクリックすると前のウィンドウに即ペーストする |
| F04 | 最大保持件数は設定で変更可能（デフォルト: 100 件） |
| F05 | 連続した同一内容は重複記録しない |
| F06 | アイテムをピン留めして削除から保護できる |
| F07 | 履歴を全件削除できる（ピン留め除く） |

#### スニペット

| ID | 要件 |
|----|------|
| F10 | 任意のテキストをタイトル付きで登録できる |
| F11 | スニペットをフォルダで分類できる |
| F12 | スニペットもホットキーメニューから即ペーストできる |
| F13 | スニペットの追加・編集・削除ができる |

#### システム

| ID | 要件 |
|----|------|
| F20 | システムトレイに常駐する |
| F21 | Windows のスタートアップに登録/解除できる |
| F22 | ホットキーは設定画面で変更できる |
| F23 | 日本語・英語の UI に対応する（システム言語で自動切替） |

### 2-2. 非機能要件

| 分類 | 要件 | 目標値 |
|------|------|--------|
| 常駐メモリ | アイドル時のプロセスメモリ | **20 MB 以下** |
| 起動速度 | 実行からトレイ表示まで | **1 秒以内** |
| メニュー表示 | ホットキー押下からメニュー表示まで | **100 ms 以内** |
| 配布サイズ | インストーラーファイルサイズ | **20 MB 以下** |
| 対応 OS | Windows 10 (1903 以降) / Windows 11 | |
| ライセンス | MIT | |

---

## 3. 技術スタック選定

### 3-1. 選定結果

**Tauri v2（Rust バックエンド）を採用する。**

| レイヤー | 採用技術 |
|----------|----------|
| フレームワーク | **Tauri v2** |
| バックエンド言語 | **Rust** |
| メインメニュー UI | **ネイティブトレイメニュー**（OS が描画、WebView 不使用） |
| 設定・スニペット UI | **WebView ウィンドウ**（HTML/CSS、開いた時のみ起動） |
| DB | **SQLite**（`tauri-plugin-sql` + `rusqlite`） |
| クリップボード監視 | `tauri-plugin-clipboard-manager` + Win32 `WM_CLIPBOARDUPDATE` |
| グローバルホットキー | `tauri-plugin-global-shortcut` |
| 多言語 | Rust の `rust-i18n` クレート |

---

### 3-2. 技術選定の背景

#### 参照した既存 OSS クリップボードマネージャー

実装に入る前に、実際に使われている技術スタックを調査した。

| アプリ | 言語・FW | 常駐メモリ | 備考 |
|--------|---------|-----------|------|
| **Ditto** | C++ + MFC | 〜5MB | Windows 最古参。最軽量の代名詞 |
| **CopyQ** | C++ + Qt6 | 〜20MB | クロスプラットフォーム |
| **PasteBar** | **Rust + Tauri + React** | 〜30〜50MB | 2024年登場。Mac/Windows 両対応 |
| **Beetroot** | **Rust + Tauri v2 + React** | 〜30〜50MB | 2025年。6MB インストーラー |

> C# / .NET を使った主要 OSS クリップボードマネージャーは存在しない。

---

#### 検討した選択肢と比較

**当初案: C# WPF**
最初に検討したが調査により棄却。

```
WPF の問題点（実測ベース）
  - 空アプリで 40〜60MB（WinForms の約 7 倍）
  - 起動時に DirectX を初期化するため遅い
  - Clipy のような OS ネイティブメニューの質感が出しにくい
```

**次案: C# WinForms**
WPF より大幅に軽量（6〜8MB）で、ContextMenuStrip がネイティブメニューと同等。  
Windows 専用アプリとして最もシンプルな実装が可能。

| 観点 | 評価 |
|------|------|
| 常駐メモリ | ✅ 6〜8MB |
| OS ネイティブメニュー | ✅ ContextMenuStrip |
| 開発難易度 | ✅ C# のみ |
| macOS 対応 | ❌ 不可 |
| 技術鮮度 | △ 枯れた技術 |

→ **Windows 専用前提なら最有力だったが、将来の macOS 対応を考慮して棄却。**

**採用: Tauri v2**

| 観点 | 評価 |
|------|------|
| 常駐メモリ（トレイのみ） | ✅ 8〜15MB（WebView 非起動時） |
| 設定画面を開いた時 | ⚠️ WebView2 起動 → 80MB 前後（一時的） |
| 設定画面を閉じた後 | ✅ 8〜15MB に戻る |
| macOS 対応 | ✅ コードを流用可能 |
| 技術鮮度 | ✅ モダン・活発（v2: 2024年10月リリース） |
| インストーラーサイズ | ✅ 〜6MB（実績: Beetroot） |

**却下: Pure Rust（Tauri なし）**

`tray-icon` + `muda`（ともに tauri-apps チーム製）+ `arboard` などを直接組み合わせる構成。

```
Pure Rust のデメリット
  - 設定画面・スニペット編集 UI を自前で作る必要がある
    → Rust のネイティブ GUI（iced, egui 等）は成熟度が低く実装コストが大
  - イベントループ・スレッド管理を全て自前で組む
  - 情報が少ない・日本語情報がほぼない
  - 同じ機能を Tauri の 2〜3 倍のコード量で書くことになる
```

> Pure Rust の軽量性は魅力だが、設定・スニペット UI の実装コストが  
> プロジェクト規模に対して過大であるため棄却。

---

#### Tauri のメモリ構造（重要な理解）

Tauri はウィンドウを持つかどうかでメモリ特性が大きく変わる。

```
【トレイのみ（アイドル時）】
  clip-win プロセス
    └── Rust ランタイム + Tauri コア  8〜15MB
        ※ WebView2 は起動していない

【設定画面を開いた時（一時的）】
  clip-win プロセス
    ├── Rust ランタイム + Tauri コア  8〜15MB
    └── WebView2（Edge）             +60〜80MB
        ※ ウィンドウを閉じると解放される
```

メインの操作（コピー→トレイメニュー→ペースト）は常に軽量。  
設定画面の重さは「設定を開いている間だけ」の一時的なものであり、許容範囲と判断。

---

#### `tray-icon` と Tauri の関係

```
Tauri の内部構造
  ├── wry        … WebView ラッパー（Chromium / WKWebView）← 重い部分
  ├── tao        … ウィンドウ管理
  ├── tray-icon  … トレイアイコン ← tauri-apps チームが開発・メンテ
  └── muda       … ネイティブメニュー ← tauri-apps チームが開発・メンテ

Pure Rust で作る場合 = tray-icon + muda を直接使う = Tauri から wry だけ除いた構成
```

---

### 3-3. 依存クレート一覧

```toml
[dependencies]
tauri            = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-sql = { version = "2", features = ["sqlite"] }
tauri-plugin-clipboard-manager = "2"
tauri-plugin-global-shortcut   = "2"
rust-i18n = "3"
serde     = { version = "1", features = ["derive"] }
serde_json = "1"
tokio     = { version = "1", features = ["full"] }
```

---

## 4. システムアーキテクチャ

### 4-1. 全体構成図

```
┌──────────────────────────────────────────────────────────┐
│                   clip-win（Tauri v2）                   │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │  main.rs  (エントリポイント)                    │    │
│  │  ・単一インスタンス制御 (Mutex)                 │    │
│  │  ・Tauri Builder で起動                         │    │
│  └───────────────────┬─────────────────────────────┘    │
│                      │                                   │
│  ┌───────────────────▼─────────────────────────────┐    │
│  │  AppState（Tauri manage で共有）                │    │
│  │  ・StorageService                               │    │
│  │  ・ClipboardWatcher                             │    │
│  └──────┬───────────────────────┬──────────────────┘    │
│         │                       │                        │
│  ┌──────▼──────┐        ┌───────▼───────────────────┐   │
│  │  Rust Core  │        │  Tauri Commands (IPC)     │   │
│  │  ・DB CRUD  │        │  ・get_history()          │   │
│  │  ・クリップ │        │  ・add_snippet()          │   │
│  │    ボード監視│        │  ・get_settings()         │   │
│  └──────┬──────┘        └───────────────────────────┘   │
│         │                                                │
│  ┌──────▼─────────────────────────────────────────────┐  │
│  │  ネイティブトレイメニュー（WebView 不使用）         │  │
│  │  TrayIconBuilder + Menu + Submenu                  │  │
│  │  ・履歴アイテム（クリックで即ペースト）            │  │
│  │  ・スニペット（フォルダ→サブメニュー）             │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  WebView ウィンドウ（設定・スニペット管理時のみ）  │  │
│  │  HTML / CSS / TypeScript（または Rust-only の場合  │  │
│  │  は egui 等ネイティブ GUI も選択肢）               │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

### 4-2. データフロー

```
【履歴記録】
  ユーザーが Ctrl+C
    → OS から WM_CLIPBOARDUPDATE（Windows）/ NSPasteboard 変更通知（macOS）
    → ClipboardWatcher がイベントを受信
    → StorageService.add_history(text)
    → トレイメニューを非同期で更新

【履歴ペースト】
  ユーザーが Ctrl+Shift+V
    → tauri-plugin-global-shortcut がイベント発火
    → AppState::show_tray_menu()
    → ネイティブメニューが表示される
    → ユーザーがアイテムをクリック
    → Clipboard::write_text(content)
    → 前のウィンドウにフォーカスを戻し Ctrl+V を送信

【設定変更】
  設定画面を開く
    → WebView ウィンドウを生成（WebView2 / WKWebView が起動）
    → フロントエンドから Tauri Command で設定を読み書き
    → ウィンドウを閉じると WebView が解放される
```

---

## 5. UI / UX 設計

### 5-1. メインメニュー（ネイティブトレイメニュー）

ホットキー押下 または トレイアイコン左クリックで表示。  
**WebView を一切使わない OS ネイティブメニュー**で描画する。

```
┌────────────────────────────────────┐
│ 📌 ピン留めされたテキスト          │  ← ピン留め（上部固定）
├────────────────────────────────────┤
│ Hello, World!                      │  ← 履歴アイテム（最新順）
│ foo@example.com                    │    クリックで即ペースト
│ https://github.com/...             │    右クリック → コンテキストメニュー
│ お疲れ様です                       │
│   ：（最大10件）                   │
├────────────────────────────────────┤
│ 履歴をすべて表示           ▶       │  ← サブメニューで全件
├────────────────────────────────────┤
│ スニペット                 ▶       │
│   ├ [📁 挨拶文]            ▶       │
│   │    ├ こんにちは                │
│   │    └ お疲れ様です             │
│   └ [📁 定型文]            ▶       │
├────────────────────────────────────┤
│ スニペットを編集...                │
│ 設定...                            │
│ 終了                               │
└────────────────────────────────────┘
```

### 5-2. 右クリックコンテキストメニュー（履歴アイテム）

```
┌────────────────────┐
│ 📌 ピン留め / 解除 │
│ 🗑 削除            │
└────────────────────┘
```

### 5-3. トレイアイコン右クリック

```
┌────────────────────────┐
│ 開く (Ctrl+Shift+V)    │
│ ──────────────────     │
│ スニペットを編集...    │
│ 設定...                │
│ ──────────────────     │
│ 終了                   │
└────────────────────────┘
```

### 5-4. 設定ウィンドウ（WebView）

```
┌─────────────────────────────────────┐
│  設定 — clip-win                    │
├─────────────────────────────────────┤
│  ホットキー       [Ctrl+Shift+V   ] │  ← 押鍵で記録
│  履歴の最大件数   [100            ] │
│  メニュー表示件数  [10            ] │
│  言語             [自動 (システム)▼]│
│  起動時に自動起動  [✓]             │
├─────────────────────────────────────┤
│                   [キャンセル] [保存]│
└─────────────────────────────────────┘
```

### 5-5. スニペット管理ウィンドウ（WebView）

```
┌──────────────────────────────────────────────┐
│  スニペット管理 — clip-win                   │
├──────────────────┬───────────────────────────┤
│ 📁 挨拶文        │  タイトル                 │
│   こんにちは     │  [こんにちは            ] │
│   お疲れ様です   │                           │
│ 📁 定型文        │  内容                     │
│   住所           │  [こんにちは、          ] │
│                  │  [よろしくお願いします。] │
│ [＋フォルダ]     │                           │
│ [＋スニペット]   │  フォルダ [挨拶文       ▼]│
│                  │       [削除]  [保存]      │
└──────────────────┴───────────────────────────┘
```

---

## 6. データベース設計

### 保存場所

```
# Windows
%APPDATA%\clip-win\clip-win.db

# macOS（将来対応）
~/Library/Application Support/clip-win/clip-win.db
```

### テーブル定義

#### `clipboard_history`

| カラム | 型 | 制約 | 説明 |
|--------|-----|------|------|
| `id` | INTEGER | PK, AUTOINCREMENT | |
| `content` | TEXT | NOT NULL | クリップボードのテキスト |
| `created_at` | TEXT | NOT NULL, DEFAULT now | 記録日時（ISO8601） |
| `is_pinned` | INTEGER | NOT NULL, DEFAULT 0 | ピン留めフラグ (0/1) |

インデックス: `(is_pinned DESC, created_at DESC)`

#### `snippet_folders`

| カラム | 型 | 制約 | 説明 |
|--------|-----|------|------|
| `id` | INTEGER | PK, AUTOINCREMENT | |
| `name` | TEXT | NOT NULL | フォルダ名 |
| `sort_order` | INTEGER | NOT NULL, DEFAULT 0 | 表示順 |

#### `snippets`

| カラム | 型 | 制約 | 説明 |
|--------|-----|------|------|
| `id` | INTEGER | PK, AUTOINCREMENT | |
| `folder_id` | INTEGER | FK（NULL = 未分類） | |
| `title` | TEXT | NOT NULL | 表示タイトル |
| `content` | TEXT | NOT NULL | ペースト内容 |
| `sort_order` | INTEGER | NOT NULL, DEFAULT 0 | |
| `created_at` | TEXT | NOT NULL, DEFAULT now | |

#### `settings`

| カラム | 型 | 制約 | 説明 |
|--------|-----|------|------|
| `key` | TEXT | PK | |
| `value` | TEXT | NOT NULL | |

##### 設定キー一覧

| key | デフォルト | 説明 |
|-----|-----------|------|
| `history_limit` | `100` | 履歴最大保持件数 |
| `menu_display_count` | `10` | メニューに直接表示する件数 |
| `hotkey` | `Ctrl+Shift+V` | グローバルホットキー |
| `language` | `` | 空=自動, `en`, `ja` |
| `startup_enabled` | `0` | Windows 起動時自動起動 |

---

## 7. モジュール設計

### ファイル構成

```
clip-win/
├── Cargo.toml
├── tauri.conf.json
├── package.json                    （設定UI を HTML で作る場合）
│
├── src-tauri/
│   ├── src/
│   │   ├── main.rs                 エントリポイント・Tauri Builder
│   │   ├── lib.rs                  Tauri setup・AppState 定義
│   │   │
│   │   ├── core/
│   │   │   ├── storage.rs          SQLite CRUD（履歴・スニペット・設定）
│   │   │   ├── clipboard.rs        クリップボード監視・自動記録
│   │   │   ├── tray.rs             トレイアイコン・メニュー構築
│   │   │   ├── paste.rs            前ウィンドウへフォーカス戻し + 自動ペースト
│   │   │   └── i18n.rs             多言語リソース管理
│   │   │
│   │   ├── models/
│   │   │   ├── clipboard_entry.rs
│   │   │   ├── snippet.rs
│   │   │   └── snippet_folder.rs
│   │   │
│   │   └── commands/               Tauri IPC コマンド（フロントエンドから呼ばれる）
│   │       ├── history.rs          get_history, delete_entry, toggle_pin, clear_history
│   │       ├── snippets.rs         get_snippets, add_snippet, update_snippet, delete_snippet
│   │       └── settings.rs         get_settings, save_settings
│   │
│   └── icons/
│       └── tray.png
│
└── src/                            設定・スニペット管理 UI（HTML/CSS/TS）
    ├── settings/
    └── snippets/
```

### 各モジュールの責務

#### `main.rs`
- Tauri アプリケーションのエントリポイント
- `Mutex` で二重起動を防止
- `tauri::Builder` でプラグイン・コマンド・AppState を登録

#### `lib.rs` / `AppState`
- `StorageService` と `ClipboardWatcher` を保持
- `tauri::Manager` の `manage()` で全ハンドラから参照可能にする

#### `core/storage.rs`
- SQLite の初期化・マイグレーション
- 履歴・スニペット・設定の CRUD
- `rusqlite` を直接使用（`tauri-plugin-sql` 経由も可）

#### `core/clipboard.rs`
- OS のクリップボード変更通知を受信
  - Windows: `WM_CLIPBOARDUPDATE`（Win32 API）
  - macOS: `NSPasteboard` のポーリング or 通知
- テキスト変更を検出して `StorageService::add_history()` を呼ぶ
- 自アプリによる書き込みを無視するためのフラグ管理

#### `core/tray.rs`
- `TrayIconBuilder` でトレイアイコンを構築
- メニューを構築する `build_menu(history, folders)` 関数
- クリップボード・スニペット更新時にメニューを再構築

#### `core/paste.rs`
- メニュー表示前のフォアグラウンドウィンドウを記憶
- アイテム選択後にクリップボードへセット
- フォーカスを元ウィンドウに戻して `Ctrl+V` を送信（`enigo` クレート使用）

#### `commands/`
- フロントエンド（設定・スニペット画面）から `invoke()` で呼ばれる Tauri コマンド群
- 実処理は `core/` モジュールに委譲

---

## 8. 開発フェーズ

### Phase 1 — コア動作（MVP）
**目標**: コピー → ホットキー → クリック → ペーストが動く

- [ ] Tauri v2 プロジェクトセットアップ
- [ ] `StorageService`：DB 初期化・履歴 CRUD
- [ ] `ClipboardWatcher`：Windows クリップボード監視・自動記録
- [ ] `tray.rs`：履歴アイテムをネイティブトレイメニューで表示
- [ ] `paste.rs`：アイテム選択 → 元ウィンドウへ自動ペースト
- [ ] グローバルホットキー（`tauri-plugin-global-shortcut`）

**完了基準**: コピー→ホットキー→クリック→ペーストの一連が動作する

---

### Phase 2 — スニペット・ピン留め
**目標**: Clipy 相当の機能を揃える

- [ ] スニペット CRUD（DB + Tauri コマンド）
- [ ] トレイメニューにスニペットサブメニューを追加
- [ ] ピン留め機能（上部固定）
- [ ] 右クリックコンテキストメニュー（ピン留め・削除）
- [ ] 履歴上限・重複排除のロジック

**完了基準**: 全機能要件 F01〜F13 を満たす

---

### Phase 3 — 設定・品質
**目標**: 実用に耐えるレベルにする

- [ ] 設定画面（WebView）：ホットキー変更・言語・起動設定
- [ ] スニペット管理画面（WebView）
- [ ] 日英ローカライズ（`rust-i18n`）
- [ ] Windows スタートアップ登録
- [ ] エラーハンドリング（クリップボードロック・DB エラー等）
- [ ] 非機能要件の計測と最適化（メモリ 20MB 以下・起動 1 秒以内）

**完了基準**: 非機能要件を満たし、日常使用に耐える

---

### Phase 4 — 配布準備
**目標**: 他の人に渡せる状態にする

- [ ] アプリアイコン（tray.png）作成
- [ ] `cargo tauri build` でインストーラー生成・動作確認
- [ ] GitHub Releases へバイナリ公開
- [ ] `CHANGELOG.md` 作成
- [ ] （将来）macOS ビルドの確認

---

## 9. ビルド・配布方針

### ビルドコマンド

```bash
# 開発・ホットリロード
cargo tauri dev

# リリースビルド（インストーラー生成）
cargo tauri build
# → src-tauri/target/release/bundle/
#      nsis/clip-win_x.x.x_x64-setup.exe   （Windows インストーラー）
#      msi/clip-win_x.x.x_x64_en-US.msi    （MSI 形式）
```

### 配布形式

| 形式 | 用途 |
|------|------|
| NSIS インストーラー（.exe） | GitHub Releases メイン |
| MSI（.msi） | 企業配布・将来用途 |
| ポータブル（単一 EXE） | `tauri.conf.json` で設定可能 |

### 動作要件（エンドユーザー向け）

- Windows 10 (1903 以降) または Windows 11
- WebView2 ランタイム（Windows 10/11 に標準搭載済みのため追加インストール不要）
- 管理者権限不要

### 想定サイズ（Beetroot の実績より）

| 項目 | 目標 |
|------|------|
| インストーラーサイズ | 〜6MB |
| インストール後サイズ | 〜15MB |
| 常駐メモリ（アイドル） | 8〜15MB |

---

## 10. 検討経緯（Decision Log）

設計に至るまでの検討過程を記録する。別環境・別セッションで再開する際の文脈として活用すること。

---

### 10-1. 出発点

- 参照アプリ: macOS の **Clipy**（[github.com/Clipy/Clipy](https://github.com/Clipy/Clipy)）
- Clipy の実態を調査した結果：
  - **Swift + AppKit** 製
  - UI は `NSMenu`（macOS OS 標準カスケードメニュー）をそのまま使用
  - **検索機能は存在しない**（当初の認識は誤りだった）
  - 軽さの本質 = カスタム UI を一切持たず、OS が描画するメニューのみ使う設計

---

### 10-2. 技術スタックの変遷

#### ① C# WPF（初期案）→ 棄却

最初に「Windows の主流デスクトップ技術」として WPF を検討。  
調査後に棄却。

**棄却理由:**
- 空アプリのメモリが 40〜60MB（WinForms の約 7 倍）
- DirectX 初期化コストで起動が遅い
- OS ネイティブメニューの質感が出しにくい
- C# を使った主要 OSS クリップボードマネージャーが存在しないことも判明

#### ② C# WinForms → 最終的に棄却

WPF 棄却後に WinForms を検討。実際に大半のコードを書いた段階で再評価。

**評価:**
- 常駐メモリ 6〜8MB ✅
- `ContextMenuStrip` が OS ネイティブメニューと同等の質感 ✅
- `ToolStripTextBox` で検索ボックスをメニュー内に 1 行埋め込める ✅
- Windows 専用（macOS 対応不可）❌

**棄却理由:** 将来的な macOS 対応の可能性を考慮して最終的に却下。

#### ③ UI 方針の整理（Clipy との認識合わせ）

コード実装後に「本家 Clipy の動作イメージを確認したい」という観点で再整理。

- Clipy に**検索機能は存在しない**ことを確認
- UI は「OS 標準カスケードメニューのみ」
- 当初設計の「カスタムポップアップウィンドウ＋検索」はオーバースペックと判断

**UI 選択肢の整理:**

| 選択肢 | 内容 |
|--------|------|
| A. Clipy 完全忠実 | 検索なし・カスケードメニューのみ |
| B. 軽量ベース＋検索 1 行 | ネイティブメニュー＋検索ボックス 1 行 |

→ 当初は **B** を選択（検索ありで進める方針）

#### ④ Tauri の検討と WebView 問題の発見

「軽量重視なら Tauri が良いのでは」という観点で Tauri を調査。

**重大な発見:**

```
検索ボックス付きポップアップ = カスタムウィンドウ = WebView2 が起動
→ メモリが 80〜120MB に膨れ上がる（軽量の目的と逆行）
```

**Tauri のメモリ構造を理解:**
- トレイ＋ネイティブメニューのみ → WebView2 起動なし → 8〜15MB ✅
- カスタムウィンドウを 1 枚でも開く → WebView2 起動 → +60〜80MB ⚠️（一時的）

#### ⑤ 「検索なし」前提での再比較

「検索なし」に絞ると WebView2 問題が消え、Tauri の選択肢が現実的になった。

| | WinForms | Tauri（トレイのみ） |
|--|---------|-------------------|
| 常駐メモリ | 6〜8MB | 8〜15MB |
| macOS 対応 | ❌ | ✅ |
| 技術鮮度 | △ | ✅ |

→ ほぼ同等の軽量性。macOS 対応と技術鮮度で Tauri に優位性あり。

#### ⑥ Tauri vs Pure Rust の比較

「Tauri か Pure Rust か」で比較。

**Pure Rust の構成:** `tray-icon`（tauri-apps 製）+ `muda`（tauri-apps 製）+ `arboard` + `rusqlite`  
※ Pure Rust = Tauri から WebView 層だけ抜いた構成に等しい

| | Tauri | Pure Rust |
|--|-------|---------|
| 設定 UI の実装 | WebView（HTML）で容易 | ネイティブ GUI が未成熟で困難 |
| フレームワーク | あり（構造が整っている） | なし（全部自前） |
| 情報量 | 多い | 少ない（日本語ほぼなし） |
| メモリ | 8〜15MB（アイドル時） | 3〜5MB |

**Pure Rust の致命的な問題:** 設定画面・スニペット編集 UI を自前で作る必要があり、  
Rust のネイティブ GUI ライブラリ（iced, egui）の成熟度が低く実装コストが過大。

#### ⑦ 最終決定

**Tauri v2（Rust バックエンド）を採用。検索は v1.0 スコープ外。**

決定の根拠:
1. 常駐メモリ 8〜15MB（非機能要件 20MB 以下を達成可能）
2. 設定 UI を WebView（HTML/CSS）で実装できる（一時的な重さは許容）
3. 将来の macOS 対応がコード流用で可能
4. tauri-apps チームが `tray-icon`/`muda` も管理しており、エコシステムが一貫している
5. 実績: Beetroot（同種アプリ）が Tauri v2 で 6MB インストーラーを達成

---

### 10-3. スコープの変遷

| 項目 | 当初 | 最終 | 理由 |
|------|------|------|------|
| 検索ボックス | v1.0 に含む | **スコープ外** | Tauri でカスタムウィンドウを使うと重くなるため |
| 画像対応 | スコープ外 | スコープ外 | 変わらず |
| macOS 対応 | 将来検討 | **ゴールに明記** | Tauri 選定理由の 1 つなので明示 |

---

## 11. 開発環境セットアップ

**対象 OS: Windows 10 / 11**（実行・テストは Windows 環境が必須）  
コードの閲覧・編集は macOS でも可能だが、`cargo tauri dev` での動作確認は Windows が必要。

---

### 11-1. 必要なツール一覧

| ツール | バージョン | 用途 |
|--------|-----------|------|
| Rust | 1.77 以上 (stable) | バックエンド実装言語 |
| Node.js | 18 以上 LTS | Tauri CLI・フロントエンドビルド |
| VS Build Tools | 2022 | Rust の Windows ビルドに必要 |
| Git | 最新 | ソース管理 |
| VS Code | 最新 | 推奨エディタ |

---

### 11-2. インストール手順（Windows）

#### Step 1: Visual Studio Build Tools

Rust のコンパイルに必要な C++ ビルドツールを先に入れる。

```
https://visualstudio.microsoft.com/visual-cpp-build-tools/
```

インストール時に **「C++ によるデスクトップ開発」** を選択。

#### Step 2: Rust

```powershell
# rustup インストーラーをダウンロード・実行
# https://rustup.rs/
winget install Rustlang.Rustup

# インストール確認
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

#### Step 5: VS Code 拡張機能（推奨）

| 拡張機能 ID | 用途 |
|------------|------|
| `rust-lang.rust-analyzer` | Rust 補完・型チェック |
| `tauri-apps.tauri-vscode` | Tauri サポート |
| `esbenp.prettier-vscode` | フロントエンド整形 |

---

### 11-3. リポジトリのクローンと初回ビルド

```powershell
# リポジトリのクローン
git clone https://github.com/tkpurine/clip-win.git
cd clip-win

# 依存パッケージのインストール（フロントエンド）
npm install

# 開発サーバー起動（初回は数分かかる）
cargo tauri dev
```

> **注意:** 初回ビルドは Rust の依存クレートをすべてコンパイルするため  
> 5〜10 分かかる場合がある。2 回目以降は差分のみでほぼ即時。

---

### 11-4. ディレクトリ構造の確認ポイント

```
clip-win/
├── docs/design.md         ← この設計書
├── src-tauri/             ← Rust バックエンド（ここがメイン）
│   ├── Cargo.toml         ← Rust 依存クレート定義
│   ├── tauri.conf.json    ← Tauri 設定（ウィンドウ・tray 設定）
│   └── src/
│       └── main.rs        ← エントリポイント
└── src/                   ← フロントエンド（設定・スニペット UI）
```

---

## 12. 次のアクション

**現在のステータス:** 設計完了・実装未着手

---

### 12-1. Phase 1 着手前の準備チェックリスト

新しい環境で再開する場合、まずこれを確認する。

```
[ ] git clone https://github.com/tkpurine/clip-win.git
[ ] Rust stable インストール済み（rustc --version で確認）
[ ] Node.js 18+ インストール済み（node --version で確認）
[ ] Tauri CLI v2 インストール済み（cargo tauri --version で確認）
[ ] VS Build Tools インストール済み（Windows のみ）
```

---

### 12-2. Phase 1 の実装ステップ（具体的な順序）

#### Step 1: Tauri v2 プロジェクト初期化

```powershell
# clip-win ディレクトリ内で実行
# ※ 既存の README.md / docs/ を残しつつ Tauri プロジェクトを初期化する
cargo create-tauri-app --template vanilla-ts --identifier com.clipwin.app
```

`tauri.conf.json` で以下を設定：
```json
{
  "app": {
    "withGlobalTauri": true
  },
  "bundle": {
    "identifier": "com.clipwin.app",
    "icon": ["icons/tray.png"]
  },
  "trayIcon": {
    "iconPath": "icons/tray.png",
    "iconAsTemplate": true
  }
}
```

#### Step 2: Cargo.toml に依存クレートを追加

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-global-shortcut = "2"
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }

[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
  "Win32_Foundation",
  "Win32_System_DataExchange",
  "Win32_UI_WindowsAndMessaging",
  "Win32_UI_Input_KeyboardAndMouse",
] }
```

#### Step 3: StorageService 実装（`src-tauri/src/core/storage.rs`）

実装する関数:
```rust
pub fn new(db_path: &str) -> Result<Self>      // DB 初期化・マイグレーション
pub fn add_history(&self, content: &str)       // 履歴追加（重複チェック込み）
pub fn get_history(&self, limit: i64) -> Vec<ClipboardEntry>
pub fn delete_history(&self, id: i64)
pub fn clear_history(&self)                    // ピン留め以外を全削除
pub fn toggle_pin(&self, id: i64)
pub fn get_setting(&self, key: &str) -> Option<String>
pub fn set_setting(&self, key: &str, value: &str)
```

#### Step 4: ClipboardWatcher 実装（`src-tauri/src/core/clipboard.rs`）

- Windows: `AddClipboardFormatListener` + `WM_CLIPBOARDUPDATE` を受信するネイティブウィンドウを作成
- 変更検出時に `StorageService::add_history()` を呼ぶ
- **自アプリによる書き込みを無視するフラグ**（`is_writing` フラグ）を必ず実装すること  
  → ペースト処理中の自己検知ループを防ぐため

#### Step 5: TrayMenu 構築（`src-tauri/src/core/tray.rs`）

```rust
pub fn build_menu(history: &[ClipboardEntry], folders: &[SnippetFolder]) -> Menu
pub fn rebuild_tray(app: &AppHandle)   // 履歴更新時に呼ぶ
```

メニュー構造:
```
MenuId("pin_{id}")  → ピン留めアイテム
--- separator ---
MenuId("hist_{id}") → 履歴アイテム（最大10件）
MenuId("hist_all")  → 「すべて表示」サブメニュー
--- separator ---
MenuId("snip_root") → スニペットサブメニュー
--- separator ---
MenuId("open_snippets") → スニペット管理画面
MenuId("open_settings") → 設定画面
MenuId("quit")          → 終了
```

#### Step 6: 自動ペースト（`src-tauri/src/core/paste.rs`）

```rust
pub fn capture_foreground() -> HWND          // メニュー表示前に呼ぶ
pub async fn paste_to(hwnd: HWND, text: &str) // クリップボードにセット→フォーカス戻し→Ctrl+V
```

#### Step 7: グローバルホットキー登録

```rust
// lib.rs の setup 内
app.handle().plugin(
  tauri_plugin_global_shortcut::Builder::new()
    .with_shortcut("Ctrl+Shift+V")?
    .with_handler(|app, shortcut, event| {
      if event.state == ShortcutState::Pressed {
        // トレイメニューを表示
      }
    })
    .build()
)?;
```

---

### 12-3. Phase 1 完了の確認方法

以下の動作が Windows 上で確認できれば Phase 1 完了。

```
1. アプリ起動 → タスクトレイにアイコンが表示される
2. 任意のテキストをコピー → トレイアイコン右クリックでメニューに追加されている
3. Ctrl+Shift+V を押す → トレイメニューが表示される
4. メニューの履歴アイテムをクリック → テキストエディタ等にペーストされる
5. アプリ終了 → 再起動後も履歴が復元されている（SQLite 永続化の確認）
```

---

### 12-4. 既知の実装上の注意点

| 注意点 | 詳細 |
|--------|------|
| クリップボード書き込みループ | ペースト時に自アプリの `ClipboardWatcher` が反応しないよう `is_writing` フラグで抑制する |
| トレイメニューの再構築コスト | 履歴が変わるたびに `rebuild_tray()` を呼ぶが、高頻度コピー時にちらつく可能性がある。デバウンス（200ms）を検討 |
| Win32 メッセージループ | `ClipboardWatcher` 用のネイティブウィンドウは専用スレッドで動かす。Tauri のメインスレッドをブロックしないこと |
| 前ウィンドウの記憶 | `capture_foreground()` はメニューを**表示する直前**に呼ぶ。表示後に呼ぶと clip-win 自身が前ウィンドウになる |
| SQLite のスレッド安全性 | `rusqlite` はデフォルトでスレッドセーフでないため `Mutex<Connection>` でラップして `AppState` に持たせる |
