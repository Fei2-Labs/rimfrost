//! Cross-session long-term memory for the rimfrost agent.
//!
//! Reads/writes memory files from `~/.rimfrost/memory/` (user-level) and
//! `.rimfrost/memory/` (project-level). Loaded into the system prompt so the
//! agent accumulates knowledge over time.

use std::fs;
use std::path::{Path, PathBuf};

const MAX_MEMORY_BUDGET: usize = 2000;

/// Resolve the user-level memory directory (`~/.rimfrost/memory/`).
fn user_memory_dir() -> Option<PathBuf> {
    std::env::var_os("RIMFROST_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".rimfrost")))
        .map(|d| d.join("memory"))
}

/// Resolve the project-level memory directory (`.rimfrost/memory/`).
fn project_memory_dir(cwd: &Path) -> PathBuf {
    cwd.join(".rimfrost").join("memory")
}

/// Read a memory file, returning empty string if missing.
fn read_memory_file(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// Load and render the long-term memory section for the system prompt.
/// Returns `None` if no memory content exists.
#[must_use]
pub fn load_memory_section(cwd: &Path) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(user_dir) = user_memory_dir() {
        let global = read_memory_file(&user_dir.join("global.md"));
        if !global.trim().is_empty() {
            parts.push(format!("## User memory\n{}", global.trim()));
        }
    }

    let project = read_memory_file(&project_memory_dir(cwd).join("project.md"));
    if !project.trim().is_empty() {
        parts.push(format!("## Project memory\n{}", project.trim()));
    }

    if parts.is_empty() {
        return None;
    }

    let mut combined = parts.join("\n\n");
    if combined.len() > MAX_MEMORY_BUDGET {
        combined.truncate(MAX_MEMORY_BUDGET);
        combined.push_str("\n\n[memory truncated]");
    }

    Some(format!("# Long-term memory\n\n{combined}"))
}

/// Write a memory entry to disk.
pub fn write_memory(
    cwd: &Path,
    scope: &str,
    category: &str,
    title: &str,
    content: &str,
) -> Result<String, String> {
    let dir = match scope {
        "user" => user_memory_dir().ok_or("cannot resolve user memory directory")?,
        "project" => project_memory_dir(cwd),
        _ => return Err(format!("unknown scope: {scope}")),
    };

    match category {
        "fact" | "preference" => {
            let file = dir.join(if scope == "user" { "global.md" } else { "project.md" });
            fs::create_dir_all(file.parent().unwrap_or(&dir)).map_err(|e| e.to_string())?;
            let mut existing = read_memory_file(&file);
            if !existing.is_empty() && !existing.ends_with('\n') {
                existing.push('\n');
            }
            existing.push_str(&format!("\n### {title}\n{content}\n"));
            fs::write(&file, existing).map_err(|e| e.to_string())?;
            Ok(format!("Saved {category} to {}", file.display()))
        }
        "skill" => {
            let skills_dir = dir.join("skills");
            fs::create_dir_all(&skills_dir).map_err(|e| e.to_string())?;
            let slug: String = title
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect();
            let file = skills_dir.join(format!("{slug}.md"));
            fs::write(&file, format!("# {title}\n\n{content}\n")).map_err(|e| e.to_string())?;
            Ok(format!("Saved skill to {}", file.display()))
        }
        _ => Err(format!("unknown category: {category}")),
    }
}

/// Search memory files for a query string (case-insensitive substring match).
pub fn search_memory(cwd: &Path, query: &str, scope: &str) -> Result<String, String> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    let search_dirs: Vec<(String, PathBuf)> = match scope {
        "user" => user_memory_dir().into_iter().map(|d| ("user".into(), d)).collect(),
        "project" => vec![("project".into(), project_memory_dir(cwd))],
        "all" | _ => {
            let mut dirs: Vec<(String, PathBuf)> = Vec::new();
            if let Some(d) = user_memory_dir() {
                dirs.push(("user".into(), d));
            }
            dirs.push(("project".into(), project_memory_dir(cwd)));
            dirs
        }
    };

    for (scope_name, dir) in search_dirs {
        if !dir.exists() {
            continue;
        }
        for entry in walkdir(&dir) {
            let content = read_memory_file(&entry);
            if content.to_lowercase().contains(&query_lower) {
                let relative = entry.strip_prefix(&dir).unwrap_or(&entry);
                results.push(format!(
                    "--- [{scope_name}] {} ---\n{}",
                    relative.display(),
                    truncate(&content, 500)
                ));
            }
        }
    }

    if results.is_empty() {
        Ok("No matching memory entries found.".into())
    } else {
        Ok(results.join("\n\n"))
    }
}

fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir(&path));
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }
    files
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...[truncated]", &s[..max])
    }
}
