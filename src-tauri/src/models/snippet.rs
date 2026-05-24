use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: i64,
    pub folder_id: Option<i64>,
    pub title: String,
    pub content: String,
    pub sort_order: i64,
    pub created_at: String,
}
