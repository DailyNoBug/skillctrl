#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::env;
use std::path::PathBuf;
use std::process::Command;

#[derive(Serialize)]
struct CommandExecution {
    success: bool,
    stdout: String,
    stderr: String,
    json: Option<Value>,
    command_line: String,
    binary_path: String,
}

#[tauri::command]
fn locate_skillctrl_binary() -> Result<String, String> {
    resolve_skillctrl_binary()
        .map(|path| path.display().to_string())
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn run_skillctrl(args: Vec<String>) -> Result<CommandExecution, String> {
    let binary = resolve_skillctrl_binary().map_err(|err| err.to_string())?;
    let output = Command::new(&binary)
        .arg("--json-resp")
        .args(&args)
        .output()
        .map_err(|err| format!("failed to launch {}: {}", binary.display(), err))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let parsed_json = parse_json_output(&stdout).or_else(|| parse_json_output(&stderr));

    Ok(CommandExecution {
        success: output.status.success(),
        stdout,
        stderr,
        json: parsed_json,
        command_line: format!("{} --json-resp {}", binary.display(), args.join(" ")),
        binary_path: binary.display().to_string(),
    })
}

fn parse_json_output(raw: &str) -> Option<Value> {
    if raw.trim().is_empty() {
        return None;
    }
    serde_json::from_str(raw).ok()
}

fn resolve_skillctrl_binary() -> Result<PathBuf> {
    if let Ok(explicit) = env::var("SKILLCTRL_BINARY") {
        let path = PathBuf::from(explicit);
        if path.exists() {
            return Ok(path);
        }
    }

    let current_exe = env::current_exe().context("failed to determine current executable path")?;
    let current_dir = current_exe
        .parent()
        .ok_or_else(|| anyhow!("current executable has no parent directory"))?;

    let sibling = current_dir.join(skillctrl_binary_name());
    if sibling.exists() && sibling != current_exe {
        return Ok(sibling);
    }

    if let Some(path) = find_on_path(skillctrl_binary_name()) {
        return Ok(path);
    }

    Err(anyhow!(
        "could not find the skillctrl binary. Place skillctrl next to skillctrl-desktop or set SKILLCTRL_BINARY."
    ))
}

fn skillctrl_binary_name() -> &'static str {
    if cfg!(windows) {
        "skillctrl.exe"
    } else {
        "skillctrl"
    }
}

fn find_on_path(binary_name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for segment in env::split_paths(&path_var) {
        let candidate = segment.join(binary_name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            locate_skillctrl_binary,
            run_skillctrl
        ])
        .run(tauri::generate_context!())
        .expect("error while running skillctrl-desktop");
}
