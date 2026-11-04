//! Which window a repository opens in: one repository per window, and a
//! repository that's already open focuses its window (PRD §6.1).

use std::path::Path;

/// The window an open request came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// Use this window if it has no repository yet (the welcome screen).
    Window(String),
    /// Use any window that has no repository yet.
    AnyEmpty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// The repository is already open in this window.
    Focus(String),
    /// Open it in this empty window.
    Reuse(String),
    NewWindow,
}

/// Decides where the repository at `root` opens. `windows` are the repository
/// windows, with the work-tree root each one shows.
pub fn place(windows: &[(&str, Option<&Path>)], root: &Path, target: &Target) -> Placement {
    if let Some((label, _)) = windows.iter().find(|(_, open)| *open == Some(root)) {
        return Placement::Focus(label.to_string());
    }
    let empty = windows.iter().find(|(label, open)| {
        open.is_none()
            && match target {
                Target::Window(target) => label == target,
                Target::AnyEmpty => true,
            }
    });
    match empty {
        Some((label, _)) => Placement::Reuse(label.to_string()),
        None => Placement::NewWindow,
    }
}

pub const LABEL_PREFIX: &str = "repo-";

/// The label for a new repository window: the lowest free `repo-N`. Reusing
/// labels means a new window gets the size and position the window with that
/// label had last time (TECH.md §4.7).
pub fn next_label<'a>(taken: impl IntoIterator<Item = &'a str> + Clone) -> String {
    (1..)
        .map(|n| format!("{LABEL_PREFIX}{n}"))
        .find(|label| !taken.clone().into_iter().any(|taken| taken == label))
        .expect("there is always a free label")
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "/repos/a";
    const B: &str = "/repos/b";

    fn place_in(windows: &[(&str, Option<&str>)], root: &str, target: Target) -> Placement {
        let windows: Vec<_> = windows
            .iter()
            .map(|(label, open)| (*label, open.map(Path::new)))
            .collect();
        place(&windows, Path::new(root), &target)
    }

    #[test]
    fn an_open_repository_focuses_its_window() {
        let windows = [("repo-1", Some(A)), ("repo-2", None)];
        assert_eq!(
            place_in(&windows, A, Target::Window("repo-2".into())),
            Placement::Focus("repo-1".into())
        );
    }

    #[test]
    fn an_empty_target_window_is_reused() {
        let windows = [("repo-1", Some(A)), ("repo-2", None)];
        assert_eq!(
            place_in(&windows, B, Target::Window("repo-2".into())),
            Placement::Reuse("repo-2".into())
        );
        assert_eq!(
            place_in(&windows, B, Target::AnyEmpty),
            Placement::Reuse("repo-2".into())
        );
    }

    #[test]
    fn a_window_showing_another_repository_opens_a_new_one() {
        let windows = [("repo-1", Some(A)), ("repo-2", None)];
        assert_eq!(
            place_in(&windows, B, Target::Window("repo-1".into())),
            Placement::NewWindow
        );
        assert_eq!(place_in(&[], A, Target::AnyEmpty), Placement::NewWindow);
    }

    #[test]
    fn labels_fill_the_lowest_gap() {
        assert_eq!(next_label([]), "repo-1");
        assert_eq!(next_label(["repo-1", "repo-3", "settings"]), "repo-2");
        assert_eq!(next_label(["repo-2", "repo-1"]), "repo-3");
    }
}
