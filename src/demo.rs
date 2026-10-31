use crate::model::{Account, ChatSession, OrgFolder};
use std::path::PathBuf;

/// Fabricated accounts used for screenshots and for trying the UI without
/// pointing it at a real `~/.claude` directory. None of this is read from or
/// written to disk.
pub fn demo_accounts() -> Vec<Account> {
    let now = now_ms();
    let hours = 3_600_000;
    let days = 24 * hours;

    let work = Account {
        account_id: "a1b2c3d4-0000-4000-8000-000000000001".to_string(),
        live_org_id: Some("org-work-0001".to_string()),
        is_current: true,
        orgs: vec![OrgFolder {
            org_id: "org-work-0001".to_string(),
            sessions: vec![
                session("acme-api", "fix flaky checkout test", now - 2 * hours, 14),
                session("acme-api", "add retry on payment webhook", now - 1 * days, 22),
                session("acme-api", "migrate rate limiter to redis", now - 4 * days, 31),
                session("dashboard-ui", "dark mode toggle", now - 5 * hours, 9),
                session("dashboard-ui", "fix chart tooltip overflow", now - 2 * days, 6),
                session("infra-scripts", "set up ci caching", now - 6 * days, 11),
                session("infra-scripts", "rotate staging secrets", now - 9 * days, 4),
            ],
        }],
    };

    let personal = Account {
        account_id: "e5f6a7b8-0000-4000-8000-000000000002".to_string(),
        live_org_id: Some("org-personal-0002".to_string()),
        is_current: false,
        orgs: vec![OrgFolder {
            org_id: "org-personal-0002".to_string(),
            sessions: vec![
                session("weekend-game-jam", "procedural terrain generation", now - 3 * hours, 27),
                session("weekend-game-jam", "fix camera jitter on slopes", now - 1 * days, 8),
                session("blog", "migrate posts to mdx", now - 3 * days, 13),
                session("notes-app", "offline sync conflict resolution", now - 7 * days, 19),
                session("notes-app", "add full text search", now - 12 * days, 10),
            ],
        }],
    };

    vec![work, personal]
}

fn session(folder: &str, title: &str, last_activity_at: i64, completed_turns: i64) -> ChatSession {
    let id = format!(
        "demo-{}",
        title
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
    );
    ChatSession {
        file_name: format!("local_{id}.json"),
        cwd: format!("/Users/demo/code/{folder}"),
        title: title.to_string(),
        last_activity_at,
        completed_turns,
        path: PathBuf::from(format!("/dev/null/{id}.json")),
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
