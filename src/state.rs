use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

/// Per-file progress entry stored on disk.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProgressEntry {
    pub chapter_index: usize,
    pub scroll_offset: usize,
    pub title: String,
    #[serde(default)]
    pub timestamp: u64,
}

/// Top-level on-disk format.
#[derive(Serialize, Deserialize, Default)]
struct ProgressDb {
    entries: HashMap<String, ProgressEntry>,
}

/// Manages reading-position persistence.
pub struct StateManager {
    db_path: PathBuf,
    db: ProgressDb,
}

impl StateManager {
    pub fn new() -> Self {
        let db_path = state_path();
        let db = load_db(&db_path).unwrap_or_default();
        Self { db_path, db }
    }

    /// Look up the saved position for `epub_path`.
    /// Returns `None` if no position was ever saved.
    pub fn load(&self, epub_path: &Path) -> Option<ProgressEntry> {
        let key = storage_key(epub_path);
        self.db.entries.get(&key).cloned()
    }

    /// Persist the current reading position to disk immediately.
    pub fn save(
        &mut self,
        epub_path: &Path,
        chapter_index: usize,
        scroll_offset: usize,
        title: &str,
    ) -> anyhow::Result<()> {
        let key = storage_key(epub_path);
        self.db.entries.insert(
            key,
            ProgressEntry {
                chapter_index,
                scroll_offset,
                title: title.to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            },
        );

        // Ensure parent directory exists
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)
                .context("failed to create state directory")?;
        }

        let json = serde_json::to_string_pretty(&self.db)
            .context("failed to serialize progress")?;
        fs::write(&self.db_path, json)
            .context("failed to write progress file")?;

        Ok(())
    }
}

// ── helpers ────────────────────────────────────────────────────────────

fn state_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("eread").join("progress.json")
}

fn load_db(path: &Path) -> Option<ProgressDb> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Build a stable key for an EPUB path.  We canonicalise so that relative,
/// absolute, and symlinked paths all map to the same entry.
fn storage_key(path: &Path) -> String {
    let canonical = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());
    canonical.to_string_lossy().to_string()
}
