use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use serde::Serialize;
use std::io::Read;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
#[error("terminal failed: {0}")]
pub struct TerminalError(String);

impl Serialize for TerminalError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

fn failed(error: impl std::fmt::Display) -> TerminalError {
    TerminalError(error.to_string())
}

fn shell_command(command: &str) -> CommandBuilder {
    if cfg!(windows) {
        let mut builder = CommandBuilder::new("cmd");
        builder.args(["/C", command]);
        builder
    } else {
        let mut builder = CommandBuilder::new("sh");
        builder.args(["-c", command]);
        builder
    }
}

pub fn spawn(
    command: &str,
    cwd: &Path,
    on_output: impl Fn(String) + Send + 'static,
    on_exit: impl FnOnce(u32) + Send + 'static,
) -> Result<(), TerminalError> {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(failed)?;
    let mut builder = shell_command(command);
    builder.cwd(cwd);
    let mut child = pair.slave.spawn_command(builder).map_err(failed)?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().map_err(failed)?;

    std::thread::spawn(move || {
        let _master = pair.master;
        let mut buffer = [0u8; 4096];
        while let Ok(read) = reader.read(&mut buffer) {
            if read == 0 {
                break;
            }
            on_output(String::from_utf8_lossy(&buffer[..read]).into_owned());
        }
        let code = child.wait().map_or(1, |status| status.exit_code());
        on_exit(code);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn streams_output_and_exit_code() {
        let (output_tx, output_rx) = mpsc::channel();
        let (exit_tx, exit_rx) = mpsc::channel();
        let dir = tempfile::tempdir().unwrap();

        spawn(
            "echo hello && exit 3",
            dir.path(),
            move |text| output_tx.send(text).unwrap(),
            move |code| exit_tx.send(code).unwrap(),
        )
        .unwrap();

        assert_eq!(exit_rx.recv_timeout(Duration::from_secs(10)).unwrap(), 3);
        let output: String = output_rx.try_iter().collect();
        assert!(output.contains("hello"));
    }

    #[test]
    fn runs_in_given_directory() {
        let (output_tx, output_rx) = mpsc::channel();
        let (exit_tx, exit_rx) = mpsc::channel();
        let dir = tempfile::tempdir().unwrap();
        let name = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let pwd = if cfg!(windows) { "cd" } else { "pwd" };

        spawn(
            pwd,
            dir.path(),
            move |text| output_tx.send(text).unwrap(),
            move |code| exit_tx.send(code).unwrap(),
        )
        .unwrap();

        exit_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let output: String = output_rx.try_iter().collect();
        assert!(output.contains(&name));
    }
}
