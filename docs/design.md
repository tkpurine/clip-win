# clipy-win 設計書

**バージョン**: 1.0  
**作成日**: 2026-05-23  
**ステータス**: Draft

---

## 目次

1. [プロジェクト概要](#1-プロジェクト概要)
2. [要件定義](#2-要件定義)
3. [技術スタック](#3-技術スタック)
4. [システムアーキテクチャ](#4-システムアーキテクチャ)
5. [UI / UX 設計](#5-ui--ux-設計)
6. [データベース設計](#6-データベース設計)
7. [モジュール設計](#7-モジュール設計)
8. [開発フェーズ](#8-開発フェーズ)
9. [ビルド・配布方針](#9-ビルド配布方針)

---

## 1. プロジェクト概要

### 背景

macOS の [Clipy](https://github.com/Clipy/Clipy) は、  
「OS ネイティブのカスケードメニューで履歴・スニペットを即呼び出す」という  
シンプルかつ軽量な設計で高い評価を受けているクリップボードマネージャー。  
同等の体験を Windows 上で実現する。

### ゴール

| # | ゴール |
|---|--------|
| G1 | コピー履歴をホットキー一発で呼び出してペーストできる |
| G2 | よく使う定型文をスニペットとして登録・即呼び出しできる |
| G3 | **軽量**（常駐メモリ 20 MB 以下、起動 1 秒以内） |
| G4 | 将来的に配布できる品質（単一 EXE、インストーラー不要） |

### スコープ外（v1.0）

- 画像クリップボード対応
- クラウド同期
- 複数デバイス共有

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
| F08 | 検索ボックスでリアルタイム絞り込みができる |

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
| 起動速度 | EXE 実行からトレイ表示まで | **1 秒以内** |
| メニュー表示 | ホットキー押下からメニュー表示まで | **100 ms 以内** |
| 配布サイズ | 単一 EXE のファイルサイズ | **30 MB 以下** |
| 対応 OS | Windows 10 (1903 以降) / Windows 11 | |
| ライセンス | MIT | |

---

## 3. 技術スタック

### 選定結果

| レイヤー | 採用技術 | 理由 |
|----------|----------|------|
| ランタイム | **.NET 8** | LTS、軽量、自己完結 EXE に対応 |
| UI フレームワーク | **WinForms** | WPF より起動・メモリが軽量。ContextMenuStrip がネイティブメニューと同等の質感 |
| トレイ | `NotifyIcon` (WinForms) | Windows 標準 API、追加依存なし |
| メニュー | `ContextMenuStrip` + `ToolStripTextBox` | 検索ボックスを1行埋め込める。サブメニュー対応 |
| DB | **SQLite** (`Microsoft.Data.Sqlite`) | ファイル1本で完結、軽量 |
| クリップボード監視 | Win32 `AddClipboardFormatListener` | ポーリング不要、OSイベント駆動 |
| ホットキー | Win32 `RegisterHotKey` | システム全体で動作 |
| 自動ペースト | Win32 `SendInput` | 確実にCtrl+Vを送れる |
| 多言語 | .NET `ResourceManager` (.resx) | 標準機能、追加ライブラリ不要 |

### WPF を選ばなかった理由

```
WPF はカスタム UI には強力だが、このアプリには過剰。
- 起動時に DirectX を初期化するため遅い
- 常駐メモリが WinForms の約 3 倍
- Clipy のような「OS ネイティブメニュー」の質感が出しにくい
```

### パッケージ一覧

```xml
<PackageReference Include="Microsoft.Data.Sqlite" Version="8.x" />
<!-- それ以外はすべて .NET 標準ライブラリのみ -->
```

依存パッケージを最小にすることで配布サイズと脆弱性リスクを抑える。

---

## 4. システムアーキテクチャ

### 全体構成図

```
┌──────────────────────────────────────────────────────────┐
│                   clipy-win.exe                          │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Program.cs  (エントリポイント)                    │  │
│  │  ・単一インスタンス制御 (Mutex)                    │  │
│  │  ・Application.Run(new AppController())            │  │
│  └──────────────────────┬─────────────────────────────┘  │
│                         │ 生成・統括                      │
│  ┌──────────────────────▼─────────────────────────────┐  │
│  │  AppController  (ApplicationContext)               │  │
│  │  ・NotifyIcon 管理                                 │  │
│  │  ・各サービスの生成・ライフサイクル管理             │  │
│  └──────┬──────────────┬──────────────┬───────────────┘  │
│         │              │              │                   │
│  ┌──────▼──────┐ ┌─────▼──────┐ ┌────▼────────────────┐  │
│  │  Clipboard  │ │  Hotkey    │ │  MenuBuilder        │  │
│  │  Watcher   │ │  Manager   │ │  (ContextMenuStrip) │  │
│  │  (Win32)   │ │  (Win32)   │ │                     │  │
│  └──────┬──────┘ └─────┬──────┘ └────┬────────────────┘  │
│         │              │              │                   │
│  ┌──────▼──────────────▼──────────────▼───────────────┐  │
│  │              StorageService  (SQLite)               │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐    │
│  │  SettingsWindow / SnippetEditorWindow (WinForms) │    │
│  └──────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

### データフロー

```
【履歴記録】
  ユーザーが Ctrl+C
    → OS から WM_CLIPBOARDUPDATE
    → ClipboardWatcher.OnClipboardChanged()
    → StorageService.AddHistory(text)

【履歴ペースト】
  ユーザーが Ctrl+Shift+V
    → Win32 WM_HOTKEY
    → HotkeyManager.OnHotkeyPressed()
    → AppController.ShowMenu()
    → ContextMenuStrip が表示される
    → ユーザーがアイテムをクリック
    → Clipboard.SetText(content)
    → AutoPasteHelper.SendCtrlV(previousWindow)
```

---

## 5. UI / UX 設計

### 5-1. メニューの全体構成

ホットキー押下 または トレイアイコン左クリックで表示。

```
┌────────────────────────────────────┐
│ [🔍 絞り込み検索...              ] │  ← ToolStripTextBox (常に表示)
├────────────────────────────────────┤
│ 📌 固定したいテキスト              │  ← ピン留めアイテム（上部固定）
├────────────────────────────────────┤
│ Hello, World!                      │  ← 履歴アイテム (最新順)
│ foo@example.com                    │    クリックで即ペースト
│ https://github.com/...             │    右クリック → コンテキストメニュー
│ お疲れ様です                       │
│   ：（最大10件表示）               │
├────────────────────────────────────┤
│ 履歴をすべて表示          ▶        │  ← サブメニューで全件
├────────────────────────────────────┤
│ スニペット                ▶        │  ← フォルダ → アイテムのサブメニュー
│   ├ [📁 挨拶文]           ▶        │
│   │    ├ こんにちは                │
│   │    └ お疲れ様です             │
│   └ [📁 定型文]           ▶        │
├────────────────────────────────────┤
│ スニペットを編集...                │
│ 設定...                            │
│ 終了                               │
└────────────────────────────────────┘
```

### 5-2. 検索の動作仕様

```
入力なし  → 通常表示（履歴最新10件 + スニペット）
入力あり  → 履歴・スニペット両方をリアルタイム絞り込み
            ・部分一致（大文字小文字を区別しない）
            ・結果0件のとき「見つかりません」を表示
```

### 5-3. 右クリックコンテキストメニュー（履歴アイテム）

```
┌────────────────────┐
│ 📌 ピン留め / 解除 │
│ 🗑 削除            │
└────────────────────┘
```

### 5-4. トレイアイコン右クリック

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

### 5-5. 設定ウィンドウ

```
┌─────────────────────────────────────┐
│  設定 — clipy-win                   │
├─────────────────────────────────────┤
│  ホットキー      [Ctrl+Shift+V    ] │  ← クリックして押鍵で記録
│  履歴の最大件数  [100             ] │
│  メニュー表示件数 [10             ] │
│  言語            [自動 (システム) ▼]│
│  起動時に自動起動 [✓]              │
├─────────────────────────────────────┤
│                   [キャンセル] [保存]│
└─────────────────────────────────────┘
```

### 5-6. スニペットエディタウィンドウ

```
┌──────────────────────────────────────────────┐
│  スニペット管理 — clipy-win                  │
├──────────────────┬───────────────────────────┤
│ 📁 挨拶文        │  タイトル                 │
│   こんにちは     │  [こんにちは            ] │
│   お疲れ様です   │                           │
│ 📁 定型文        │  内容                     │
│   住所           │  [こんにちは、           ]│
│                  │  [よろしくお願いします。 ]│
│ [＋フォルダ]     │                           │
│ [＋スニペット]   │  フォルダ  [挨拶文      ▼]│
│                  │                           │
│                  │       [削除] [保存]       │
└──────────────────┴───────────────────────────┘
```

---

## 6. データベース設計

### 保存場所

```
%APPDATA%\clipy-win\clipy-win.db
```

### テーブル定義

#### `clipboard_history`

| カラム | 型 | 制約 | 説明 |
|--------|-----|------|------|
| `id` | INTEGER | PK, AUTOINCREMENT | |
| `content` | TEXT | NOT NULL | クリップボードのテキスト内容 |
| `created_at` | TEXT | NOT NULL, DEFAULT now | 記録日時（ISO8601） |
| `is_pinned` | INTEGER | NOT NULL, DEFAULT 0 | ピン留めフラグ (0/1) |

インデックス: `created_at DESC`（検索・表示の高速化）

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
| `folder_id` | INTEGER | FK → snippet_folders(id) ON DELETE SET NULL | NULL = フォルダなし |
| `title` | TEXT | NOT NULL | 表示タイトル |
| `content` | TEXT | NOT NULL | ペースト内容 |
| `sort_order` | INTEGER | NOT NULL, DEFAULT 0 | フォルダ内表示順 |
| `created_at` | TEXT | NOT NULL, DEFAULT now | |

#### `settings`

| カラム | 型 | 制約 | 説明 |
|--------|-----|------|------|
| `key` | TEXT | PK | 設定キー |
| `value` | TEXT | NOT NULL | 設定値 |

##### 設定キー一覧

| key | デフォルト値 | 説明 |
|-----|-------------|------|
| `history_limit` | `100` | 履歴最大保持件数 |
| `menu_display_count` | `10` | メニューに直接表示する履歴件数 |
| `hotkey_modifiers` | `0x4006` | Ctrl+Shift + NOREPEAT |
| `hotkey_vk` | `0x56` | 'V' |
| `hotkey_display` | `Ctrl+Shift+V` | 設定画面表示用 |
| `language` | `` | 空=自動, `en`, `ja` |
| `startup_enabled` | `0` | Windows 起動時自動起動 |

---

## 7. モジュール設計

### ファイル構成

```
clipy-win/
├── clipy-win.csproj
├── Program.cs                   エントリポイント・単一インスタンス制御
├── AppController.cs             ApplicationContext・サービス統括・トレイ管理
│
├── Core/
│   ├── ClipboardWatcher.cs      Win32 WM_CLIPBOARDUPDATE フック
│   ├── HotkeyManager.cs         Win32 RegisterHotKey / UnregisterHotKey
│   ├── StorageService.cs        SQLite CRUD（履歴・スニペット・設定）
│   ├── AutoPasteHelper.cs       前ウィンドウへフォーカス戻し + SendInput
│   └── LocalizationService.cs  ResourceManager ラッパー (Loc.T("key"))
│
├── Models/
│   ├── ClipboardEntry.cs
│   ├── Snippet.cs
│   └── SnippetFolder.cs
│
├── UI/
│   ├── MenuBuilder.cs           ContextMenuStrip の組み立て・フィルタリング
│   ├── SettingsForm.cs          設定フォーム
│   ├── SnippetEditorForm.cs     スニペット管理フォーム
│   └── InputDialog.cs           汎用テキスト入力ダイアログ
│
└── Resources/
    ├── Strings.resx             英語リソース
    ├── Strings.ja.resx          日本語リソース
    └── tray.ico                 トレイアイコン
```

### 各クラスの責務

#### `Program.cs`
- アプリケーションエントリポイント
- `Mutex` で二重起動を防止
- `Application.Run(new AppController())` で起動

#### `AppController : ApplicationContext`
- `NotifyIcon` の生成・管理
- `ClipboardWatcher`・`HotkeyManager`・`StorageService` の生成
- ホットキー押下時に `MenuBuilder.ShowMenu()` を呼び出す
- アプリ終了時のクリーンアップ

#### `ClipboardWatcher`
- Win32 `AddClipboardFormatListener` でクリップボード変更を監視
- `WM_CLIPBOARDUPDATE` 受信時に `ClipboardChanged` イベントを発火
- メッセージ受信用のネイティブウィンドウ（`NativeWindow`）を内部に保持

#### `HotkeyManager`
- `RegisterHotKey` / `UnregisterHotKey` のラッパー
- `WM_HOTKEY` 受信時に `HotkeyPressed` イベントを発火

#### `StorageService`
- SQLite 接続・スキーママイグレーション
- 履歴・スニペット・設定の CRUD を提供
- 全メソッドを同期で実装（メインスレッドから呼ぶため）

#### `MenuBuilder`
- `ContextMenuStrip` の構築と表示
- `ToolStripTextBox`（検索ボックス）のテキスト変更時にメニューアイテムを再描画
- 検索フィルタリングロジックを担当

#### `AutoPasteHelper`
- `GetForegroundWindow` でメニュー表示前のウィンドウを記憶
- `SetForegroundWindow` でフォーカスを戻す
- `SendInput` で `Ctrl+V` を送信

---

## 8. 開発フェーズ

### Phase 1 — コア動作（MVP）
**目標**: 最低限動く状態にする

- [ ] プロジェクトセットアップ（.csproj、フォルダ構成）
- [ ] `StorageService`：DB 初期化・履歴 CRUD
- [ ] `ClipboardWatcher`：クリップボード監視・履歴自動記録
- [ ] `MenuBuilder`：履歴をシンプルなメニューで表示
- [ ] `AppController`：トレイアイコン・クリックでメニュー表示
- [ ] `AutoPasteHelper`：選択したアイテムをペースト
- [ ] `HotkeyManager`：ホットキーでメニュー表示

**完了基準**: コピー→ホットキー→クリック→ペーストが動く

---

### Phase 2 — スニペット＋検索
**目標**: Clipy 相当の機能を揃える

- [ ] `MenuBuilder`：スニペットサブメニュー追加
- [ ] `MenuBuilder`：検索ボックス（`ToolStripTextBox`）追加・フィルタリング
- [ ] `SnippetEditorForm`：スニペット管理 UI
- [ ] `StorageService`：スニペット CRUD
- [ ] ピン留め機能・右クリックコンテキストメニュー

**完了基準**: 全機能要件 F01〜F13 を満たす

---

### Phase 3 — 設定・品質
**目標**: 実用に耐えるレベルにする

- [ ] `SettingsForm`：ホットキー変更・履歴件数・言語選択
- [ ] `LocalizationService`：日英リソース実装
- [ ] Windows スタートアップ登録（レジストリ）
- [ ] エラーハンドリング（クリップボードロック、DB エラー等）
- [ ] メモリ・起動速度の計測と最適化

**完了基準**: 非機能要件（メモリ 20MB 以下・起動 1 秒以内）を満たす

---

### Phase 4 — 配布準備
**目標**: 他の人に渡せる状態にする

- [ ] トレイアイコン画像作成（`tray.ico`）
- [ ] 単一 EXE ビルドの確認（`dotnet publish -r win-x64 --self-contained`）
- [ ] GitHub Releases へのバイナリ公開
- [ ] `CHANGELOG.md` 作成

---

## 9. ビルド・配布方針

### ビルドコマンド

```bash
# 開発・デバッグ
dotnet run

# リリースビルド（単一 EXE、自己完結）
dotnet publish -c Release -r win-x64 \
  --self-contained true \
  -p:PublishSingleFile=true \
  -p:IncludeNativeLibrariesForSelfExtract=true
# → bin/Release/net8.0-windows/win-x64/publish/clipy-win.exe
```

### 配布形式

| 形式 | 用途 |
|------|------|
| 単一 EXE（自己完結） | 個人利用・GitHub Releases |
| ZIP（EXE 同梱） | 初回配布 |
| （将来）NSIS インストーラー | 広く配布する場合 |

### 動作要件（エンドユーザー向け）

- Windows 10 (1903 以降) または Windows 11
- **.NET ランタイム不要**（自己完結 EXE のため）
- 管理者権限不要
