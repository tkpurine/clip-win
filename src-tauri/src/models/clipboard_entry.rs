use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: i64,
    pub content: String,
    pub created_at: String,
    pub is_pinned: bool,
}

impl ClipboardEntry {
    /// 表示用にテキストを最大 `max_len` 文字に切り詰める
    pub fn display_title(&self, max_len: usize) -> String {
        // 改行を空白に置換して 1 行表示にする
        let single_line = self.content.replace('\n', " ").replace('\r', "");
        let trimmed = single_line.trim().to_string();
        if trimmed.chars().count() <= max_len {
            trimmed
        } else {
            let truncated: String = trimmed.chars().take(max_len).collect();
            format!("{}…", truncated)
        }
    }
}
