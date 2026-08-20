use std::ffi::OsStr;
use std::process::Command;

#[cfg(windows)]
pub fn command(program: impl AsRef<OsStr>) -> Command {
    use std::os::windows::process::CommandExt;

    let mut command = Command::new(program);
    command.creation_flags(0x08000000);
    command
}

#[cfg(not(windows))]
pub fn command(program: impl AsRef<OsStr>) -> Command {
    Command::new(program)
}
