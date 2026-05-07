use arboard::Clipboard;
use std::process::Command;
use tokio::time::{sleep, Duration};

pub async fn capture_selected_text() -> Result<String, SelectionError> {
    log::debug!("selection capture started");
    let mut clipboard = Clipboard::new()?;
    let previous = clipboard.get_text().ok();
    log::debug!(
        "clipboard snapshot captured; had_text={}",
        previous.is_some()
    );

    send_copy_shortcut()?;
    log::debug!("copy shortcut sent; waiting for clipboard update");
    sleep(Duration::from_millis(180)).await;

    let selected = Clipboard::new()?.get_text()?.trim().to_string();
    log::debug!(
        "clipboard selected text read; char_count={}",
        selected.chars().count()
    );

    if let Some(previous) = previous {
        let _ = Clipboard::new()?.set_text(previous);
        log::debug!("clipboard text restored");
    }

    if selected.is_empty() {
        log::warn!("selection capture produced empty text");
        return Err(SelectionError::EmptySelection);
    }

    Ok(selected)
}

fn send_copy_shortcut() -> Result<(), SelectionError> {
    #[cfg(target_os = "macos")]
    {
        log::debug!("sending macOS copy shortcut through osascript");
        run_command(
            "osascript",
            &[
                "-e",
                r#"tell application "System Events" to keystroke "c" using command down"#,
            ],
        )
    }

    #[cfg(target_os = "windows")]
    {
        log::debug!("sending Windows copy shortcut through PowerShell SendKeys");
        run_command(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('^c')",
            ],
        )
    }

    #[cfg(target_os = "linux")]
    {
        log::debug!("sending Linux copy shortcut through xdotool");
        if run_command("xdotool", &["key", "ctrl+c"]).is_ok() {
            return Ok(());
        }
        log::debug!("xdotool copy shortcut failed; trying wtype");
        run_command("wtype", &["-M", "ctrl", "c", "-m", "ctrl"])
    }
}

fn run_command(program: &str, args: &[&str]) -> Result<(), SelectionError> {
    let status = Command::new(program).args(args).status()?;
    if status.success() {
        log::debug!("copy command succeeded; program={program}");
        Ok(())
    } else {
        log::warn!("copy command failed; program={program} status={status}");
        Err(SelectionError::CopyCommandFailed(program.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SelectionError {
    #[error("无法访问剪贴板：{0}")]
    Clipboard(#[from] arboard::Error),
    #[error("无法执行复制快捷键：{0}")]
    Io(#[from] std::io::Error),
    #[error("复制命令执行失败：{0}")]
    CopyCommandFailed(String),
    #[error("没有读取到选中文本，请确认已经选中文本并授予必要权限")]
    EmptySelection,
}
