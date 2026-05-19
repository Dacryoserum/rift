use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileId {
    pub inode: u64,
    pub size: u64,
    pub mtime_secs: i64,
}

impl FileId {
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let meta = std::fs::metadata(path)?;

        #[cfg(unix)]
        let inode = meta.ino();
        #[cfg(not(unix))]
        let inode = 0u64;

        #[cfg(unix)]
        let mtime_secs = meta.mtime();
        #[cfg(not(unix))]
        let mtime_secs = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        Ok(Self {
            inode,
            size: meta.len(),
            mtime_secs,
        })
    }

    pub fn persist_key(&self) -> String {
        format!("{}_{}_{}", self.inode, self.size, self.mtime_secs)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub line_num: u64,
    pub byte_offset: u64,
    pub created_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct BookmarkFile {
    marks: HashMap<String, Bookmark>,
}

pub struct BookmarkStore {
    _file_id: FileId,
    marks: HashMap<char, Bookmark>,
    persist_path: PathBuf,
    dirty: bool,
}

impl BookmarkStore {
    pub fn load(file_id: FileId) -> Self {
        let persist_path = Self::persist_path(&file_id);
        let marks = Self::load_from_disk(&persist_path).unwrap_or_default();
        Self {
            _file_id: file_id,
            marks,
            persist_path,
            dirty: false,
        }
    }

    fn load_from_disk(path: &Path) -> Option<HashMap<char, Bookmark>> {
        let content = std::fs::read_to_string(path).ok()?;
        let file: BookmarkFile = toml::from_str(&content).ok()?;
        let marks = file
            .marks
            .into_iter()
            .filter_map(|(k, v)| {
                let ch = k.chars().next()?;
                Some((ch, v))
            })
            .collect();
        Some(marks)
    }

    pub fn set(&mut self, key: char, line_num: u64, byte_offset: u64) {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.marks.insert(
            key,
            Bookmark {
                line_num,
                byte_offset,
                created_at,
            },
        );
        self.dirty = true;
    }

    pub fn get(&self, key: char) -> Option<&Bookmark> {
        self.marks.get(&key)
    }

    pub fn all(&self) -> impl Iterator<Item = (char, &Bookmark)> {
        self.marks.iter().map(|(&k, v)| (k, v))
    }

    pub fn remove(&mut self, key: char) {
        if self.marks.remove(&key).is_some() {
            self.dirty = true;
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if !self.dirty && self.persist_path.exists() {
            return Ok(());
        }
        if let Some(parent) = self.persist_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let marks: HashMap<String, Bookmark> = self
            .marks
            .iter()
            .map(|(&k, v)| (k.to_string(), v.clone()))
            .collect();
        let file = BookmarkFile { marks };
        let content = toml::to_string(&file)?;
        std::fs::write(&self.persist_path, content)?;
        Ok(())
    }

    pub fn persist_path(file_id: &FileId) -> PathBuf {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share"))
            })
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("rift")
            .join(format!("{}.toml", file_id.persist_key()))
    }
}
