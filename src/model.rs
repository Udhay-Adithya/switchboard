use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ChatSession {
    pub file_name: String, // local_<uuid>.json
    pub cwd: String,
    pub title: String,
    pub last_activity_at: i64, // ms epoch
    pub completed_turns: i64,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct OrgFolder {
    pub org_id: String,
    pub sessions: Vec<ChatSession>,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub account_id: String,
    pub orgs: Vec<OrgFolder>,
    pub live_org_id: Option<String>,
    pub is_current: bool,
}

impl Account {
    /// All sessions across all org folders, tagged with which org folder they live in.
    pub fn all_sessions(&self) -> Vec<(&OrgFolder, &ChatSession)> {
        self.orgs
            .iter()
            .flat_map(|o| o.sessions.iter().map(move |s| (o, s)))
            .collect()
    }

    pub fn find_session(&self, file_name: &str) -> Option<(&OrgFolder, &ChatSession)> {
        self.all_sessions()
            .into_iter()
            .find(|(_, s)| s.file_name == file_name)
    }
}

pub fn short_uuid(s: &str) -> String {
    s.chars().take(8).collect()
}
