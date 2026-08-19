use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::paths::{write_string, Paths};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub timestamp: String,
    pub files: Vec<FileEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub original: String,
    pub backup: String,
    /// File did not exist before apply; rollback should delete it.
    pub created: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct History {
    pub last: Option<String>,
    pub sessions: Vec<String>,
}

/// One apply session. First write of each path is snapshotted.
pub struct Session {
    pub id: String,
    dir: PathBuf,
    manifest: Manifest,
    seen: std::collections::HashSet<PathBuf>,
}

impl Session {
    pub fn begin(paths: &Paths) -> Result<Self> {
        let id = timestamp_id();
        let dir = paths.backups_dir().join(&id);
        fs::create_dir_all(&dir).map_err(|e| Error::io(e, &dir))?;
        Ok(Self {
            id: id.clone(),
            dir,
            manifest: Manifest {
                id,
                timestamp: iso8601(),
                files: Vec::new(),
            },
            seen: std::collections::HashSet::new(),
        })
    }

    /// Snapshot `path` before first mutation this session.
    pub fn backup_file(&mut self, path: &Path) -> Result<()> {
        if self.seen.contains(path) {
            return Ok(());
        }
        self.seen.insert(path.to_path_buf());
        let created = !path.exists();
        let rel = sanitize(path);
        let dest = self.dir.join(&rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(e, parent))?;
        }
        if !created {
            if path.is_file() {
                fs::copy(path, &dest).map_err(|e| Error::io(e, path))?;
            }
        } else {
            // marker file so rollback knows to delete
            write_string(&dest.with_extension("created"), "")?;
        }
        self.manifest.files.push(FileEntry {
            original: path.display().to_string(),
            backup: dest.display().to_string(),
            created,
        });
        Ok(())
    }

    pub fn finish(&self, paths: &Paths) -> Result<()> {
        let man = serde_json::to_string_pretty(&self.manifest)
            .map_err(|e| Error::parse("backup manifest", e))?;
        write_string(&self.dir.join("manifest.json"), &man)?;
        let hist_path = paths.history_file();
        let mut hist = load_history(paths).unwrap_or(History {
            last: None,
            sessions: Vec::new(),
        });
        hist.last = Some(self.id.clone());
        hist.sessions.push(self.id.clone());
        if hist.sessions.len() > 32 {
            hist.sessions.drain(0..hist.sessions.len() - 32);
        }
        let raw = serde_json::to_string_pretty(&hist).map_err(|e| Error::parse("history", e))?;
        write_string(&hist_path, &raw)?;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.manifest.files.is_empty()
    }
}

pub fn rollback(paths: &Paths) -> Result<Vec<String>> {
    let hist = load_history(paths)
        .ok_or_else(|| Error::user("no apply history — nothing to roll back"))?;
    let id = hist
        .last
        .ok_or_else(|| Error::user("no apply history — nothing to roll back"))?;
    let dir = paths.backups_dir().join(&id);
    let man_path = dir.join("manifest.json");
    let raw = fs::read_to_string(&man_path).map_err(|e| Error::io(e, &man_path))?;
    let man: Manifest =
        serde_json::from_str(&raw).map_err(|e| Error::parse("backup manifest", e))?;
    let mut done = Vec::new();
    for f in man.files.iter().rev() {
        let original = PathBuf::from(&f.original);
        if f.created {
            if original.exists() {
                fs::remove_file(&original).map_err(|e| Error::io(e, &original))?;
            }
            done.push(format!("removed {}", f.original));
            continue;
        }
        let backup = PathBuf::from(&f.backup);
        if backup.is_file() {
            if let Some(parent) = original.parent() {
                fs::create_dir_all(parent).map_err(|e| Error::io(e, parent))?;
            }
            fs::copy(&backup, &original).map_err(|e| Error::io(e, &original))?;
            done.push(format!("restored {}", f.original));
        }
    }
    Ok(done)
}

pub fn load_history(paths: &Paths) -> Option<History> {
    let raw = fs::read_to_string(paths.history_file()).ok()?;
    serde_json::from_str(&raw).ok()
}

fn sanitize(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    let s = s.trim_start_matches('/');
    PathBuf::from(s.replace(':', "_"))
}

fn timestamp_id() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn iso8601() -> String {
    // Keep this free of extra crates: unix seconds as a sortable stamp.
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markers::{patch, CommentStyle};
    use tempfile::TempDir;

    #[test]
    fn rollback_restores_fixture() {
        let tmp = TempDir::new().unwrap();
        let paths = Paths::isolated(tmp.path().to_path_buf());
        paths.ensure_dirs().unwrap();
        let file = tmp.path().join("config/app.conf");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "user = 1\n").unwrap();

        let mut sess = Session::begin(&paths).unwrap();
        sess.backup_file(&file).unwrap();
        let patched = patch("user = 1\n", "generated = 2", CommentStyle::Hash);
        fs::write(&file, patched).unwrap();
        sess.finish(&paths).unwrap();

        assert!(fs::read_to_string(&file).unwrap().contains("generated = 2"));
        rollback(&paths).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "user = 1\n");
    }
}
