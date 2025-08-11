use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardTriggerQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]  
    pub per_page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_desc: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_exact: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_archived: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_flow: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

impl StandardTriggerQuery {
    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(0);
        let per_page = self.per_page.unwrap_or(100);
        (page * per_page) as i64
    }

    pub fn limit(&self) -> i64 {
        self.per_page.unwrap_or(100) as i64
    }

    pub fn order_field(&self) -> &str {
        self.order_by.as_deref().unwrap_or("edited_at")
    }
}

impl Default for StandardTriggerQuery {
    fn default() -> Self {
        Self {
            page: Some(0),
            per_page: Some(100),
            order_desc: Some(true),
            created_by: None,
            path: None,
            path_start: None,
            path_exact: None,
            first_kind: None,
            last_kind: None,
            starred_only: None,
            show_archived: None,
            order_by: Some("edited_at".to_string()),
            is_flow: None,
            enabled: None,
        }
    }
}