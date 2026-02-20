use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItemAddedEvent {
    pub id: i64,
    pub preview_text: String,
    pub created_at: i64,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClipboardPausedChangedEvent {
    pub paused: bool,
}
