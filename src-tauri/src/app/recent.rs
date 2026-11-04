//! The recent repositories list (PRD §6.1): most recent first, at most
//! [`MAX_RECENT`] entries, deduplicated by work-tree path.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

pub const MAX_RECENT: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecentRepo {
    pub path: PathBuf,
    /// The name of the work-tree folder.
    pub name: String,
    /// False when the path no longer exists. The entry is kept, shown as
    /// unavailable, until it's removed.
    pub available: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecentList {
    paths: Vec<PathBuf>,
}

impl RecentList {
    /// Moves `path` to the front, adding it if it's new.
    pub fn push(&mut self, path: &Path) {
        self.paths.retain(|existing| existing != path);
        self.paths.insert(0, path.to_owned());
        self.paths.truncate(MAX_RECENT);
    }

    /// Returns whether `path` was in the list.
    pub fn remove(&mut self, path: &Path) -> bool {
        let len = self.paths.len();
        self.paths.retain(|existing| existing != path);
        self.paths.len() != len
    }

    #[cfg(test)]
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// The entries, with `available` from `exists`.
    pub fn items(&self, exists: impl Fn(&Path) -> bool) -> Vec<RecentRepo> {
        self.paths
            .iter()
            .map(|path| RecentRepo {
                path: path.clone(),
                name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string()),
                available: exists(path),
            })
            .collect()
    }

    pub fn to_stored(&self) -> Value {
        Value::Array(
            self.paths
                .iter()
                .filter_map(|path| path.to_str())
                .map(|path| Value::String(path.to_owned()))
                .collect(),
        )
    }

    /// Reads the stored list, skipping anything that isn't an absolute path.
    pub fn from_stored(stored: Option<&Value>) -> Self {
        let mut list = Self::default();
        let paths = stored.and_then(Value::as_array).into_iter().flatten();
        for path in paths.filter_map(Value::as_str).map(Path::new) {
            if path.is_absolute() && !list.paths.iter().any(|existing| existing == path) {
                list.paths.push(path.to_owned());
            }
        }
        list.paths.truncate(MAX_RECENT);
        list
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn list(paths: &[&str]) -> RecentList {
        let mut list = RecentList::default();
        for path in paths.iter().rev() {
            list.push(Path::new(path));
        }
        list
    }

    #[test]
    fn opening_again_moves_to_the_front_without_duplicates() {
        let mut recent = list(&["/a", "/b", "/c"]);
        recent.push(Path::new("/c"));
        assert_eq!(recent, list(&["/c", "/a", "/b"]));
        recent.push(Path::new("/d"));
        assert_eq!(recent, list(&["/d", "/c", "/a", "/b"]));
    }

    #[test]
    fn keeps_at_most_ten() {
        let mut recent = RecentList::default();
        for i in 0..15 {
            recent.push(Path::new(&format!("/repo{i}")));
        }
        assert_eq!(recent.paths().len(), MAX_RECENT);
        assert_eq!(recent.paths()[0], Path::new("/repo14"));
        assert_eq!(recent.paths()[9], Path::new("/repo5"));
    }

    #[test]
    fn remove_reports_whether_it_was_there() {
        let mut recent = list(&["/a", "/b"]);
        assert!(recent.remove(Path::new("/a")));
        assert!(!recent.remove(Path::new("/a")));
        assert_eq!(recent, list(&["/b"]));
    }

    #[test]
    fn missing_paths_are_unavailable() {
        let recent = list(&["/home/me/project", "/gone"]);
        let items = recent.items(|path| path != Path::new("/gone"));
        assert_eq!(
            items,
            vec![
                RecentRepo {
                    path: "/home/me/project".into(),
                    name: "project".into(),
                    available: true,
                },
                RecentRepo {
                    path: "/gone".into(),
                    name: "gone".into(),
                    available: false,
                },
            ]
        );
    }

    #[test]
    fn stored_list_round_trips_and_skips_bad_entries() {
        let recent = list(&["/a", "/b"]);
        assert_eq!(RecentList::from_stored(Some(&recent.to_stored())), recent);
        let stored = json!(["/a", 3, "relative", "/a", "/b"]);
        assert_eq!(RecentList::from_stored(Some(&stored)), recent);
        assert_eq!(
            RecentList::from_stored(Some(&json!({}))),
            RecentList::default()
        );
    }
}
