use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};

pub async fn voice_listing() -> Option<String> {
    let output = Command::new("/usr/bin/say")
        .args(["-v", "?"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

pub async fn speak(voice: Option<&str>, text: &str) -> std::io::Result<Child> {
    let mut command = Command::new("/usr/bin/say");
    if let Some(voice) = voice {
        command.args(["-v", voice]);
    }
    let mut child = command
        .args(["-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).await?;
    }
    Ok(child)
}
