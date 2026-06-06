//! Snowflake Cortex Code CLI (`cortex`) filter module.
//!
//! Cortex Code is an interactive AI coding assistant. Most invocations are
//! interactive sessions that pass through unfiltered. This module filters
//! non-interactive commands (`--version`, `update`, `mcp list`) where
//! structured output benefits from token reduction.

use crate::core::runner;
use crate::core::utils::resolved_command;
use anyhow::{Context, Result};

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            while let Some(&nc) = chars.peek() {
                chars.next();
                if nc.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn run(args: &[String], verbose: u8) -> Result<i32> {
    if args.is_empty() {
        return run_passthrough(args, verbose);
    }

    if args.iter().any(|a| a == "--version" || a == "-V") {
        return run_version(verbose);
    }

    match args[0].as_str() {
        "update" => run_passthrough(args, verbose),
        "mcp" if args.len() >= 2 && args[1] == "list" => run_mcp_list(verbose),
        _ => run_passthrough(args, verbose),
    }
}

fn run_version(verbose: u8) -> Result<i32> {
    let timer = crate::core::tracking::TimedExecution::start();
    let mut cmd = resolved_command("cortex");
    cmd.arg("--version");

    let output = cmd
        .output()
        .context("Failed to run cortex --version")?;

    if !output.status.success() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        return Ok(output.status.code().unwrap_or(1));
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let cleaned = strip_ansi(raw.trim());

    // cortex --version outputs something like "0.42.0" or "Cortex Code CLI x.y.z"
    // Extract just the version number
    let version = cleaned
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            // Match lines like "0.42.0" or "Cortex Code CLI 0.42.0"
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            parts.last().and_then(|last| {
                if last.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                    Some(*last)
                } else {
                    None
                }
            })
        })
        .unwrap_or(&cleaned);

    if verbose > 0 {
        eprintln!("cortex version: {} (raw: {})", version, cleaned);
    }
    println!("{}", version);

    timer.track("cortex --version", "rtk cortex --version", &raw, version);
    Ok(0)
}

fn run_mcp_list(verbose: u8) -> Result<i32> {
    let timer = crate::core::tracking::TimedExecution::start();
    let mut cmd = resolved_command("cortex");
    cmd.args(["mcp", "list"]);

    let output = cmd
        .output()
        .context("Failed to run cortex mcp list")?;

    if !output.status.success() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        timer.track_passthrough("cortex mcp list", "rtk cortex mcp list (passthrough)");
        return Ok(output.status.code().unwrap_or(1));
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let raw_stripped = strip_ansi(raw.trim());

    // cortex mcp list output format (JSON-like or table):
    // Try to compact: show server name + status only
    let filtered = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw_stripped) {
        compact_mcp_json(&json)
    } else {
        // Text fallback: just show the raw output (already short)
        raw_stripped.to_string()
    };

    if verbose > 0 {
        eprintln!("cortex mcp list filtered: {}", filtered);
    }
    println!("{}", filtered);

    timer.track("cortex mcp list", "rtk cortex mcp list", &raw, &filtered);
    Ok(0)
}

fn compact_mcp_json(json: &serde_json::Value) -> String {
    if let Some(servers) = json.as_array() {
        let lines: Vec<String> = servers
            .iter()
            .filter_map(|s| {
                let name = s.get("name")?.as_str()?;
                let status = s
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                Some(format!("  {}  {}", status, name))
            })
            .collect();
        if lines.is_empty() {
            "No MCP servers configured".to_string()
        } else {
            lines.join("\n")
        }
    } else if let Some(obj) = json.as_object() {
        // Alternative format: {"servers": [...]}
        if let Some(servers) = obj.get("servers").and_then(|v| v.as_array()) {
            let lines: Vec<String> = servers
                .iter()
                .filter_map(|s| {
                    let name = s.get("name")?.as_str()?;
                    let status = s
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    Some(format!("  {}  {}", status, name))
                })
                .collect();
            if lines.is_empty() {
                "No MCP servers configured".to_string()
            } else {
                lines.join("\n")
            }
        } else {
            serde_json::to_string_pretty(json).unwrap_or_default()
        }
    } else {
        serde_json::to_string_pretty(json).unwrap_or_default()
    }
}

fn run_passthrough(args: &[String], verbose: u8) -> Result<i32> {
    let os_args: Vec<std::ffi::OsString> = args.iter().map(|s| s.into()).collect();
    runner::run_passthrough("cortex", &os_args, verbose)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi() {
        assert_eq!(strip_ansi("hello"), "hello");
        assert_eq!(strip_ansi("\x1b[32mhello\x1b[0m"), "hello");
        assert_eq!(strip_ansi("\x1b[1;32mhello\x1b[0m"), "hello");
        assert_eq!(strip_ansi("no color"), "no color");
    }

    #[test]
    fn test_version_detection() {
        let cleaned = "Cortex Code CLI 0.42.0";
        let version = cleaned
            .lines()
            .find_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                parts.last().and_then(|last| {
                    if last.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                        Some(*last)
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(cleaned);
        assert_eq!(version, "0.42.0");
    }

    #[test]
    fn test_compact_mcp_json_array() {
        let json = serde_json::json!([
            {"name": "snowflake", "status": "connected"},
            {"name": "github", "status": "disconnected"},
        ]);
        let result = compact_mcp_json(&json);
        assert!(result.contains("connected  snowflake"));
        assert!(result.contains("disconnected  github"));
    }

    #[test]
    fn test_compact_mcp_json_object() {
        let json = serde_json::json!({
            "servers": [
                {"name": "snowflake", "status": "connected"},
            ]
        });
        let result = compact_mcp_json(&json);
        assert!(result.contains("connected  snowflake"));
    }

    #[test]
    fn test_compact_mcp_json_empty() {
        let json = serde_json::json!([]);
        let result = compact_mcp_json(&json);
        assert_eq!(result, "No MCP servers configured");
    }
}
