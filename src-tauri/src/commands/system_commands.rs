//! OS-integration actions that are inherently platform-specific:
//! "Reveal Log File" (show a path in the system file manager) and
//! "Open Terminal Here" (launch a system terminal in a given directory,
//! WITHOUT pre-filling or injecting any command — the user types whatever
//! they want themselves). Both are argv-only subprocess launches, never
//! shell string interpolation, and both are explicit user-triggered actions
//! — nothing here runs automatically.

use std::path::Path;
use std::process::Command;
use tbb_core::diagnostics::DiagnosticsReport;
use tbb_core::system::run_tool_checks;

#[tauri::command]
pub fn reveal_path_in_file_manager(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("path does not exist: {path}"));
    }
    let result = if cfg!(target_os = "macos") {
        Command::new("open").arg("-R").arg(&path).spawn()
    } else if cfg!(target_os = "windows") {
        Command::new("explorer").arg("/select,").arg(&path).spawn()
    } else {
        // Most Linux file managers accept a directory to open; fall back to
        // the containing directory since not all support "select this file".
        let target = if p.is_dir() {
            p.to_path_buf()
        } else {
            p.parent()
                .map(|d| d.to_path_buf())
                .unwrap_or_else(|| p.to_path_buf())
        };
        Command::new("xdg-open").arg(target).spawn()
    };
    result
        .map(|_| ())
        .map_err(|e| format!("could not open file manager: {e}"))
}

#[tauri::command]
pub fn open_terminal_here(working_dir: String) -> Result<(), String> {
    let dir = Path::new(&working_dir);
    if !dir.is_dir() {
        return Err(format!("not a directory: {working_dir}"));
    }

    if cfg!(target_os = "macos") {
        return Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .arg(dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("could not open terminal: {e}"));
    }

    if cfg!(target_os = "windows") {
        return Command::new("cmd")
            .args(["/C", "start", "cmd.exe"])
            .current_dir(dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("could not open terminal: {e}"));
    }

    // No single canonical terminal binary on Linux; try common ones in
    // order and report clearly (naming every attempt) if none are found,
    // rather than silently doing nothing.
    let candidates = [
        "x-terminal-emulator",
        "gnome-terminal",
        "konsole",
        "xfce4-terminal",
        "alacritty",
        "kitty",
        "xterm",
    ];
    let mut last_err: Option<std::io::Error> = None;
    for candidate in candidates {
        match Command::new(candidate).current_dir(dir).spawn() {
            Ok(_) => return Ok(()),
            Err(e) => last_err = Some(e),
        }
    }
    Err(format!(
        "no supported terminal emulator found on PATH (tried: {}); last error: {}",
        candidates.join(", "),
        last_err.map(|e| e.to_string()).unwrap_or_default()
    ))
}

#[tauri::command]
pub fn run_diagnostic_tool_checks() -> Vec<tbb_core::system::ToolCheck> {
    run_tool_checks()
}

#[tauri::command]
pub fn save_diagnostics_json(report: DiagnosticsReport, destination: String) -> Result<(), String> {
    let text = tbb_core::diagnostics::to_json_pretty(&report).map_err(|e| e.to_string())?;
    std::fs::write(&destination, text).map_err(|e| e.to_string())
}
