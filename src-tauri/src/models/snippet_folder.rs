use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetFolder {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
}
