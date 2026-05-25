use rusqlite::{params, Connection, Result};
use std::sync::Mutex;

use crate::models::{
    clipboard_entry::ClipboardEntry,
    snippet::Snippet,
    snippet_folder::SnippetFolder,
};

/// SQLite を使ったデータ永続化サービス。
/// `Connection` は `Mutex` でラップしてスレッドセーフにする。
pub struct StorageService {
    conn: Mutex<Connection>,
}

impl StorageService {
    /// DB を開き、スキーマを初期化する。
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let service = StorageService {
            conn: Mutex::new(conn),
        };
        service.init_schema()?;
        Ok(service)
    }

    // -----------------------------------------------------------------------
    // スキーマ初期化・マイグレーション
    // -----------------------------------------------------------------------

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS clipboard_history (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                content    TEXT    NOT NULL,
                created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                is_pinned  INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_history_order
                ON clipboard_history (is_pinned DESC, created_at DESC);

            CREATE TABLE IF NOT EXISTS snippet_folders (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                name       TEXT    NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS snippets (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                folder_id  INTEGER,
                title      TEXT    NOT NULL,
                content    TEXT    NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                FOREIGN KEY (folder_id) REFERENCES snippet_folders(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- デフォルト設定（存在しない場合のみ挿入）
            INSERT OR IGNORE INTO settings (key, value) VALUES ('history_limit',       '100');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('menu_display_count',  '10');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('hotkey',              'Ctrl+Shift+V');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('language',            '');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('startup_enabled',     '0');
            ",
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // 履歴 CRUD
    // -----------------------------------------------------------------------

    /// 履歴を追加する。
    /// - 空文字・空白のみのテキストは無視する。
    /// - 直前の非ピン留め履歴と同一内容なら重複として無視する（F05）。
    /// - 上限件数を超えた場合、古い非ピン留めエントリを削除する（F04）。
    pub fn add_history(&self, content: &str) -> Result<()> {
        if content.trim().is_empty() {
            return Ok(());
        }

        let limit = self
            .get_setting("history_limit")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(100);

        let mut conn = self.conn.lock().unwrap();

        // 直前エントリとの重複チェック
        let recent: Option<String> = conn
            .query_row(
                "SELECT content FROM clipboard_history
                 WHERE is_pinned = 0
                 ORDER BY created_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        if recent.as_deref() == Some(content) {
            return Ok(());
        }

        // INSERT + DELETE をトランザクションで実行（クラッシュ時の整合性保証）
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO clipboard_history (content, created_at, is_pinned)
             VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 0)",
            params![content],
        )?;
        tx.execute(
            "DELETE FROM clipboard_history
             WHERE is_pinned = 0
               AND id NOT IN (
                   SELECT id FROM clipboard_history
                   WHERE is_pinned = 0
                   ORDER BY created_at DESC
                   LIMIT ?1
               )",
            params![limit],
        )?;
        tx.commit()?;

        Ok(())
    }

    /// 履歴を取得する。ピン留め優先、次いで新しい順。
    pub fn get_history(&self, limit: i64) -> Vec<ClipboardEntry> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT id, content, created_at, is_pinned
             FROM clipboard_history
             ORDER BY is_pinned DESC, created_at DESC
             LIMIT ?1",
        ) {
            Ok(s) => s,
            Err(e) => {
                log::error!("get_history prepare 失敗: {}", e);
                return Vec::new();
            }
        };

        match stmt.query_map(params![limit], |row| {
            Ok(ClipboardEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                is_pinned: row.get::<_, i64>(3)? != 0,
            })
        }) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                log::error!("get_history query 失敗: {}", e);
                Vec::new()
            }
        }
    }

    /// 指定 ID の履歴エントリを1件取得する。
    pub fn get_history_by_id(&self, id: i64) -> Option<ClipboardEntry> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, content, created_at, is_pinned
             FROM clipboard_history
             WHERE id = ?1",
            params![id],
            |row| {
                Ok(ClipboardEntry {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    created_at: row.get(2)?,
                    is_pinned: row.get::<_, i64>(3)? != 0,
                })
            },
        )
        .ok()
    }

    /// 指定 ID の履歴エントリを削除する。
    pub fn delete_history(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM clipboard_history WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// ピン留め以外の履歴を全件削除する（F07）。
    pub fn clear_history(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM clipboard_history WHERE is_pinned = 0",
            [],
        )?;
        Ok(())
    }

    /// ピン留め状態をトグルする（F06）。
    pub fn toggle_pin(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE clipboard_history
             SET is_pinned = CASE WHEN is_pinned = 0 THEN 1 ELSE 0 END
             WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // スニペット CRUD（Phase 2 で本格利用予定）
    // -----------------------------------------------------------------------

    pub fn get_snippet_folders(&self) -> Vec<SnippetFolder> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, sort_order FROM snippet_folders ORDER BY sort_order ASC",
            )
            .unwrap();

        stmt.query_map([], |row| {
            Ok(SnippetFolder {
                id: row.get(0)?,
                name: row.get(1)?,
                sort_order: row.get(2)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    pub fn get_snippets_by_folder(&self, folder_id: Option<i64>) -> Vec<Snippet> {
        let conn = self.conn.lock().unwrap();

        match folder_id {
            Some(id) => {
                let mut stmt = match conn.prepare(
                    "SELECT id, folder_id, title, content, sort_order, created_at
                     FROM snippets WHERE folder_id = ?1 ORDER BY sort_order ASC",
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("get_snippets_by_folder prepare 失敗: {}", e);
                        return Vec::new();
                    }
                };
                match stmt.query_map(params![id], |row| {
                    Ok(Snippet {
                        id: row.get(0)?,
                        folder_id: row.get(1)?,
                        title: row.get(2)?,
                        content: row.get(3)?,
                        sort_order: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                }) {
                    Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                    Err(e) => {
                        log::error!("get_snippets_by_folder query 失敗: {}", e);
                        Vec::new()
                    }
                }
            }
            None => {
                let mut stmt = match conn.prepare(
                    "SELECT id, folder_id, title, content, sort_order, created_at
                     FROM snippets WHERE folder_id IS NULL ORDER BY sort_order ASC",
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("get_snippets_by_folder(null) prepare 失敗: {}", e);
                        return Vec::new();
                    }
                };
                match stmt.query_map([], |row| {
                    Ok(Snippet {
                        id: row.get(0)?,
                        folder_id: row.get(1)?,
                        title: row.get(2)?,
                        content: row.get(3)?,
                        sort_order: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                }) {
                    Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                    Err(e) => {
                        log::error!("get_snippets_by_folder(null) query 失敗: {}", e);
                        Vec::new()
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // 設定
    // -----------------------------------------------------------------------

    pub fn get_setting(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .ok()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }
}
