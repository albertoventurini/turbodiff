//! `turbodiff [PATH] [--base REF] [--compare REF]` (PRD §6.1, TECH.md §4.7).
//!
//! The same parser handles this process's arguments and those a second launch
//! forwards to the running app.

use std::path::{Path, PathBuf};

use clap::Parser;
use turbodiff_repo::{CompareSide, Comparison};

#[derive(Debug, Parser)]
#[command(
    name = "turbodiff",
    version,
    about = "A read-only viewer for git diffs"
)]
struct Args {
    /// A folder in the repository to open. Without it, the welcome screen
    /// is shown.
    path: Option<PathBuf>,
    /// The base ref: any revision git understands. Defaults to HEAD.
    #[arg(long, value_name = "REF", requires = "path")]
    base: Option<String>,
    /// The ref to compare with the base. Defaults to the working tree.
    #[arg(long, value_name = "REF", requires = "path")]
    compare: Option<String>,
}

/// What a launch asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// The welcome screen.
    Welcome,
    Open {
        /// Absolute, resolved against the launch's working directory.
        path: PathBuf,
        /// `None` when neither `--base` nor `--compare` was given.
        comparison: Option<Comparison>,
    },
}

/// Parses `argv` (including the program name) from a launch in `cwd`.
pub fn parse(
    argv: impl IntoIterator<Item = impl Into<std::ffi::OsString> + Clone>,
    cwd: &Path,
) -> Result<Launch, clap::Error> {
    Ok(resolve(Args::try_parse_from(argv)?, cwd))
}

fn resolve(args: Args, cwd: &Path) -> Launch {
    let Some(path) = args.path else {
        return Launch::Welcome;
    };
    let comparison = (args.base.is_some() || args.compare.is_some()).then(|| {
        let default = Comparison::default();
        Comparison {
            base: args.base.unwrap_or(default.base),
            compare: args
                .compare
                .map_or(CompareSide::WorkTree, |spec| CompareSide::Commit { spec }),
            ..default
        }
    });
    Launch::Open {
        path: cwd.join(path),
        comparison,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch(args: &[&str]) -> Launch {
        let argv = std::iter::once("turbodiff").chain(args.iter().copied());
        parse(argv, Path::new("/home/me")).unwrap()
    }

    #[test]
    fn no_path_shows_the_welcome_screen() {
        assert_eq!(launch(&[]), Launch::Welcome);
    }

    #[test]
    fn paths_resolve_against_the_working_directory() {
        assert_eq!(
            launch(&["."]),
            Launch::Open {
                path: "/home/me/.".into(),
                comparison: None,
            }
        );
        assert_eq!(
            launch(&["/srv/repo"]),
            Launch::Open {
                path: "/srv/repo".into(),
                comparison: None,
            }
        );
    }

    #[test]
    fn refs_preselect_the_comparison() {
        let Launch::Open { comparison, .. } = launch(&["repo", "--base", "main"]) else {
            panic!("expected Open");
        };
        assert_eq!(
            comparison,
            Some(Comparison {
                base: "main".into(),
                ..Comparison::default()
            })
        );
        let Launch::Open { comparison, .. } =
            launch(&["repo", "--compare", "feature~2", "--base", "v1.0"])
        else {
            panic!("expected Open");
        };
        assert_eq!(
            comparison,
            Some(Comparison {
                base: "v1.0".into(),
                compare: CompareSide::Commit {
                    spec: "feature~2".into()
                },
                ..Comparison::default()
            })
        );
    }

    #[test]
    fn refs_need_a_path() {
        let argv = ["turbodiff", "--base", "main"];
        assert!(parse(argv, Path::new("/")).is_err());
    }

    #[test]
    fn unknown_flags_are_errors() {
        assert!(parse(["turbodiff", "--frobnicate"], Path::new("/")).is_err());
    }
}
