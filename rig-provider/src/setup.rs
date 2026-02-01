use anyhow::Context;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for the setup process.
pub struct SetupConfig {
    /// Whether to run in dry-run mode (no changes applied).
    pub dry_run: bool,
}

/// Runs the setup process to register the provider in CLI configs.
///
/// # Errors
/// Returns an error if environment variables are missing or if config files cannot be read/written.
pub fn run_setup(config: &SetupConfig) -> anyhow::Result<()> {
    tracing::info!("Starting Zero-Config self-registration...");

    let exe_path = std::env::current_exe()?;
    let exe_path_str = exe_path.to_string_lossy().to_string();
    let home = std::env::var("HOME").context("HOME env var not set")?;

    // 1. JSON-based configurations (Claude Code, OpenCode)
    let claude_path = PathBuf::from(&home).join(".claude.json");
    setup_json_mcp(
        "Claude Code",
        &claude_path,
        &exe_path_str,
        "rig-provider",
        config,
    )?;

    let opencode_path = PathBuf::from(&home).join(".opencode.json");
    setup_json_mcp(
        "OpenCode",
        &opencode_path,
        &exe_path_str,
        "rig-provider",
        config,
    )?;

    // 2. TOML-based configurations (Codex)
    let codex_path = PathBuf::from(&home).join(".codex/config.toml");
    setup_codex(&codex_path, &exe_path_str, "rig-provider", config)?;

    if config.dry_run {
        println!("\n[DRY RUN] Setup complete. No files were modified.");
    } else {
        println!("\n[SUCCESS] Rig Provider successfully registered for all supported CLIs.");
    }

    Ok(())
}

fn setup_json_mcp(
    name: &str,
    path: &Path,
    exe_path: &str,
    provider_name: &str,
    config: &SetupConfig,
) -> anyhow::Result<()> {
    println!("Checking {name} config at: {}", path.display());

    let mut data = if path.exists() {
        let content = fs::read_to_string(path)?;
        serde_json::from_str::<Value>(&content).unwrap_or_else(|_| serde_json::json!({"mcpServers": {}}))
    } else {
        serde_json::json!({"mcpServers": {}})
    };

    // Ensure mcpServers is an object
    if data.get("mcpServers").is_none() {
        if let Some(obj) = data.as_object_mut() {
            obj.insert("mcpServers".to_string(), serde_json::json!({}));
        }
    }

    let servers = data
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .context(format!(
            "Invalid {name} config: mcpServers must be an object"
        ))?;

    servers.insert(
        provider_name.to_string(),
        serde_json::json!({
            "command": exe_path,
            "args": [],
            "env": {}
        }),
    );

    if config.dry_run {
        println!(
            "[DRY RUN] Would update {} with {provider_name}",
            path.display()
        );
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(&data)?)?;
        println!("[OK] Registered in {name}.");
    }

    Ok(())
}

fn setup_codex(
    path: &Path,
    exe_path: &str,
    provider_name: &str,
    config: &SetupConfig,
) -> anyhow::Result<()> {
    println!("Checking Codex config at: {}", path.display());

    let mut content = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    let section_header = format!("[mcp_servers.{provider_name}]");
    if content.contains(&section_header) {
        println!("[SKIP] {provider_name} already exists in Codex config.");
    } else {
        let entry = format!("\n{section_header}\ncommand = \"{exe_path}\"\nargs = []\n");
        content.push_str(&entry);

        if config.dry_run {
            println!(
                "[DRY RUN] Would update {} with {provider_name}",
                path.display()
            );
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, content)?;
            println!("[OK] Registered in Codex.");
        }
    }

    Ok(())
}
