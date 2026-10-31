use crate::model::{Account, ChatSession, OrgFolder};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

fn claude_support_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/Application Support/Claude")
}

fn sessions_root() -> PathBuf {
    claude_support_dir().join("claude-code-sessions")
}

pub fn sessions_dir(account_id: &str, org_id: &str) -> PathBuf {
    sessions_root().join(account_id).join(org_id)
}

fn main_log_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/Logs/Claude/main.log")
}

/// Reads config.json and returns the account id that is currently logged in,
/// if we can determine it.
pub fn last_known_account_uuid() -> Option<String> {
    let path = claude_support_dir().join("config.json");
    let data = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&data).ok()?;
    v.get("lastKnownAccountUuid")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
}

/// Scans Claude Desktop's own log for the most recent successful
/// "LocalSessionManager" initialization per accountId, which tells us which
/// orgId folder is the one that account can actually authenticate against
/// right now. An org folder that never shows up here is a dead leftover from
/// a previous login and the app will refuse to read it, so a copy must land
/// in the live one to actually show up in the sidebar.
pub fn detect_live_orgs() -> HashMap<String, String> {
    let mut live: HashMap<String, String> = HashMap::new();
    let path = main_log_path();
    let Ok(file) = fs::File::open(&path) else {
        return live;
    };
    let reader = BufReader::new(file);
    for line in reader.lines().map_while(Result::ok) {
        if !line.contains("Initialization succeeded") || !line.contains("LocalSessionManager") {
            continue;
        }
        let (Some(account_id), Some(org_id)) = (
            extract_field(&line, "accountId="),
            extract_field(&line, "orgId="),
        ) else {
            continue;
        };
        // Log is chronological; the last match wins.
        live.insert(account_id, org_id);
    }
    live
}

fn extract_field(line: &str, key: &str) -> Option<String> {
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest.find(',').unwrap_or(rest.len());
    let val = rest[..end].trim();
    if val.is_empty() || val == "null" {
        None
    } else {
        Some(val.to_string())
    }
}

fn parse_session_file(path: &Path) -> Option<ChatSession> {
    let data = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&data).ok()?;
    Some(ChatSession {
        file_name: path.file_name()?.to_string_lossy().to_string(),
        cwd: v
            .get("cwd")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        title: v
            .get("title")
            .and_then(|x| x.as_str())
            .unwrap_or("(no title)")
            .to_string(),
        last_activity_at: v
            .get("lastActivityAt")
            .and_then(|x| x.as_i64())
            .unwrap_or(0),
        completed_turns: v
            .get("completedTurns")
            .and_then(|x| x.as_i64())
            .unwrap_or(0),
        path: path.to_path_buf(),
    })
}

pub fn scan_accounts() -> Vec<Account> {
    let root = sessions_root();
    let current = last_known_account_uuid();
    let live_orgs = detect_live_orgs();

    let mut accounts = Vec::new();
    let Ok(entries) = fs::read_dir(&root) else {
        return accounts;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let account_id = entry.file_name().to_string_lossy().to_string();
        let mut orgs = Vec::new();
        if let Ok(org_entries) = fs::read_dir(&path) {
            for org_entry in org_entries.flatten() {
                let org_path = org_entry.path();
                if !org_path.is_dir() {
                    continue;
                }
                let org_id = org_entry.file_name().to_string_lossy().to_string();
                let mut sessions = Vec::new();
                if let Ok(files) = fs::read_dir(&org_path) {
                    for f in files.flatten() {
                        let fp = f.path();
                        let name = f.file_name().to_string_lossy().to_string();
                        if name.starts_with("local_") && name.ends_with(".json") {
                            if let Some(s) = parse_session_file(&fp) {
                                sessions.push(s);
                            }
                        }
                    }
                }
                sessions.sort_by(|a, b| b.last_activity_at.cmp(&a.last_activity_at));
                orgs.push(OrgFolder { org_id, sessions });
            }
        }
        let live_org_id = live_orgs.get(&account_id).cloned();
        let is_current = current.as_deref() == Some(account_id.as_str());
        accounts.push(Account {
            account_id,
            orgs,
            live_org_id,
            is_current,
        });
    }
    accounts.sort_by(|a, b| b.is_current.cmp(&a.is_current));
    accounts
}
