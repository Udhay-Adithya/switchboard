use std::fs;
use std::path::{Path, PathBuf};

pub enum CopyOutcome {
    Copied,
    Updated,
    SkippedUpToDate,
}

/// Copies a session pointer file into a destination org folder.
///
/// Never touches the underlying .jsonl transcript: that file is shared
/// storage on disk regardless of which account is logged in, so only the
/// small per-account index pointer ever needs to move.
///
/// If the destination already has a pointer for the same session, the
/// richer one wins (more completed turns, or a later last-activity time) so
/// a session continued under one account after being copied never regresses
/// to a stale, shorter copy.
pub fn copy_session(
    src_path: &Path,
    dest_dir: &Path,
    src_turns: i64,
    src_last_activity: i64,
) -> Result<CopyOutcome, String> {
    let file_name = src_path
        .file_name()
        .ok_or_else(|| "source path has no file name".to_string())?;
    let dest_path = dest_dir.join(file_name);

    fs::create_dir_all(dest_dir)
        .map_err(|e| format!("could not create destination folder: {e}"))?;

    if dest_path.exists() {
        let (dest_turns, dest_la) = read_turns_and_activity(&dest_path).unwrap_or((0, 0));
        if src_turns > dest_turns || src_last_activity > dest_la {
            fs::copy(src_path, &dest_path).map_err(|e| format!("copy failed: {e}"))?;
            return Ok(CopyOutcome::Updated);
        }
        return Ok(CopyOutcome::SkippedUpToDate);
    }

    fs::copy(src_path, &dest_path).map_err(|e| format!("copy failed: {e}"))?;
    Ok(CopyOutcome::Copied)
}

fn read_turns_and_activity(path: &Path) -> Option<(i64, i64)> {
    let data = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&data).ok()?;
    let turns = v
        .get("completedTurns")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    let la = v
        .get("lastActivityAt")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    Some((turns, la))
}

pub fn format_relative_time(ms: i64) -> String {
    if ms <= 0 {
        return "unknown".to_string();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let diff_s = (now - ms) / 1000;
    if diff_s < 0 {
        return "just now".to_string();
    }
    let mins = diff_s / 60;
    let hours = mins / 60;
    let days = hours / 24;
    if days > 0 {
        format!("{days}d ago")
    } else if hours > 0 {
        format!("{hours}h ago")
    } else if mins > 0 {
        format!("{mins}m ago")
    } else {
        "just now".to_string()
    }
}

pub fn path_leaf_label(cwd: &str) -> String {
    PathBuf::from(cwd)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| cwd.to_string())
}
