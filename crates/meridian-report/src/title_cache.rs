//! Small on-disk cache for `summarize_title`: same (model, text) -> same title.
//!
//! One JSON file per entry, named by a stable 64-bit FNV-1a hash of the key. Each entry
//! stores the full key (`model`, `text`) and a hit is only returned when both match
//! exactly, so a hash collision can never return another prompt's title.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Set to `off`, `0`, `false` or `no` to disable the title cache.
pub const CACHE_TOGGLE_ENV: &str = "MERIDIAN_TITLE_CACHE";
/// Overrides the cache root (default: `$XDG_CACHE_HOME/meridian` or `~/.cache/meridian`).
pub const CACHE_DIR_ENV: &str = "MERIDIAN_CACHE_DIR";

#[derive(Serialize, Deserialize)]
struct Entry {
    model: String,
    text: String,
    title: String,
}

pub struct TitleCache {
    dir: PathBuf,
}

impl TitleCache {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The cache configured by the environment, or `None` when disabled or when no
    /// home/cache directory can be determined.
    pub fn from_env() -> Option<Self> {
        if let Ok(v) = std::env::var(CACHE_TOGGLE_ENV) {
            if matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "off" | "0" | "false" | "no"
            ) {
                return None;
            }
        }
        let root = non_empty_env(CACHE_DIR_ENV)
            .map(PathBuf::from)
            .or_else(|| non_empty_env("XDG_CACHE_HOME").map(|d| PathBuf::from(d).join("meridian")))
            .or_else(|| {
                non_empty_env("HOME")
                    .or_else(|| non_empty_env("USERPROFILE"))
                    .map(|h| PathBuf::from(h).join(".cache").join("meridian"))
            })?;
        Some(Self::new(root.join("titles")))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path_for(&self, model: &str, text: &str) -> PathBuf {
        self.dir.join(format!("{:016x}.json", fnv1a64(model, text)))
    }

    /// Cached title for exactly this (model, text), if any. Unreadable or corrupt
    /// entries are treated as misses.
    pub fn get(&self, model: &str, text: &str) -> Option<String> {
        let raw = std::fs::read_to_string(self.path_for(model, text)).ok()?;
        let entry: Entry = serde_json::from_str(&raw).ok()?;
        (entry.model == model && entry.text == text && !entry.title.trim().is_empty())
            .then_some(entry.title)
    }

    /// Store a title. Written to a sibling temp file then renamed so concurrent
    /// readers/writers never observe a partial entry.
    pub fn put(&self, model: &str, text: &str, title: &str) -> Result<()> {
        if title.trim().is_empty() {
            return Ok(());
        }
        std::fs::create_dir_all(&self.dir)
            .with_context(|| format!("creating title cache dir {}", self.dir.display()))?;
        let path = self.path_for(model, text);
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        let body = serde_json::to_string(&Entry {
            model: model.to_string(),
            text: text.to_string(),
            title: title.to_string(),
        })?;
        std::fs::write(&tmp, body)
            .with_context(|| format!("writing title cache entry {}", tmp.display()))?;
        std::fs::rename(&tmp, &path).with_context(|| {
            let _ = std::fs::remove_file(&tmp);
            format!("installing title cache entry {}", path.display())
        })?;
        Ok(())
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

/// FNV-1a 64 over `model \0 text`. Stable across Rust releases (unlike `DefaultHasher`).
fn fnv1a64(model: &str, text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in model.bytes().chain(std::iter::once(0)).chain(text.bytes()) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_hit_and_miss() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = TitleCache::new(tmp.path().join("titles"));
        assert_eq!(cache.get("m", "fix login"), None);
        cache.put("m", "fix login", "Fix login bug").unwrap();
        assert_eq!(
            cache.get("m", "fix login").as_deref(),
            Some("Fix login bug")
        );
        // Different model or text is a different key.
        assert_eq!(cache.get("other", "fix login"), None);
        assert_eq!(cache.get("m", "fix logout"), None);
    }

    #[test]
    fn colliding_file_with_different_key_is_not_returned() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = TitleCache::new(tmp.path());
        // Plant an entry for another key under this key's file name.
        let planted = Entry {
            model: "m".into(),
            text: "someone else's prompt".into(),
            title: "Wrong title".into(),
        };
        std::fs::write(
            cache.path_for("m", "my prompt"),
            serde_json::to_string(&planted).unwrap(),
        )
        .unwrap();
        assert_eq!(cache.get("m", "my prompt"), None);
    }

    #[test]
    fn corrupt_entry_is_a_miss_and_can_be_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = TitleCache::new(tmp.path());
        std::fs::write(cache.path_for("m", "t"), "{not json").unwrap();
        assert_eq!(cache.get("m", "t"), None);
        cache.put("m", "t", "Title").unwrap();
        assert_eq!(cache.get("m", "t").as_deref(), Some("Title"));
    }

    #[test]
    fn blank_titles_are_never_stored() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = TitleCache::new(tmp.path().join("titles"));
        cache.put("m", "t", "  ").unwrap();
        assert!(!cache.dir().exists());
        assert_eq!(cache.get("m", "t"), None);
    }

    #[test]
    fn put_leaves_no_temp_files() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = TitleCache::new(tmp.path());
        cache.put("m", "t", "Title").unwrap();
        let names: Vec<String> = std::fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 1);
        assert!(names[0].ends_with(".json"));
    }

    #[test]
    fn fnv_is_stable() {
        // Pinned so cache files written by one release stay addressable by the next.
        assert_eq!(fnv1a64("m", "fix login"), 0xc8d0_7e7d_8860_13ae);
    }
}
