//! Detaching from the terminal (PRD §6.1, TECH.md §4.7): `turbodiff .` gives
//! the prompt back at once, and the app outlives the terminal.
//!
//! A launch from a terminal starts a copy of itself in a new session, with no
//! controlling terminal and its standard streams on `/dev/null`, and exits.
//! The copy isn't attached to a terminal, so it runs the app itself.

use std::ffi::OsString;
use std::io::IsTerminal;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Starts the app detached and exits, when this process was launched from a
/// terminal. Otherwise, or if the copy can't be started, returns and the app
/// runs in this process.
pub fn detach() {
    let attached = std::io::stdin().is_terminal()
        || std::io::stdout().is_terminal()
        || std::io::stderr().is_terminal();
    if !attached {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let mut command = Command::new(program(exe, std::env::var_os("APPIMAGE")));
    command
        .args(std::env::args_os().skip(1))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: `setsid` is async-signal-safe, and nothing else runs between
    // `fork` and `exec`.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    if command.spawn().is_ok() {
        std::process::exit(0);
    }
}

/// The program to start. An AppImage's executable lives in a mount that goes
/// away when the launching process exits, so it's the AppImage file itself
/// (`$APPIMAGE`) that's started again.
fn program(exe: PathBuf, appimage: Option<OsString>) -> PathBuf {
    appimage
        .filter(|path| !path.is_empty())
        .map_or(exe, PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_this_executable_again() {
        let exe = PathBuf::from("/usr/bin/turbodiff");
        assert_eq!(program(exe.clone(), None), exe);
        assert_eq!(program(exe.clone(), Some(OsString::new())), exe);
    }

    #[test]
    fn starts_the_appimage_rather_than_its_mounted_executable() {
        let exe = PathBuf::from("/tmp/.mount_turboXYZ/usr/bin/turbodiff");
        let appimage = OsString::from("/home/me/TurboDiff.AppImage");
        assert_eq!(
            program(exe, Some(appimage)),
            PathBuf::from("/home/me/TurboDiff.AppImage")
        );
    }
}
