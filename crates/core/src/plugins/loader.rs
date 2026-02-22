use std::path::Path;

use anyhow::Result;

use super::manifest::PluginManifest;
use crate::skills::Skill;

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub skills: Vec<Skill>,
    pub agent_files: Vec<String>,
    pub hook_file: Option<String>,
    pub mcp_config: Option<String>,
}

/// Load a single plugin from its directory.
pub fn load_plugin(plugin_root: &Path) -> Result<LoadedPlugin> {
    let manifest = PluginManifest::load(plugin_root)?;

    let mut skills = Vec::new();
    if let Some(ref skills_path) = manifest.skills {
        let skills_dir = manifest.resolve_path(plugin_root, skills_path);
        if skills_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&skills_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let skill_file = path.join("SKILL.md");
                        if skill_file.exists() {
                            if let Ok(content) = std::fs::read_to_string(&skill_file) {
                                let name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                skills.push(Skill {
                                    name,
                                    content,
                                    path: skill_file,
                                    description: None,
                                });
                            }
                        }
                    } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let name = path
                                .file_stem()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            skills.push(Skill {
                                name,
                                content,
                                path: path.clone(),
                                description: None,
                            });
                        }
                    }
                }
            }
        }
    }

    let mut agent_files = Vec::new();
    if let Some(ref agents_dirs) = manifest.agents {
        for agents_path in agents_dirs {
            let agents_dir = manifest.resolve_path(plugin_root, agents_path);
            if agents_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&agents_dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.ends_with(".md") {
                                agent_files.push(entry.path().display().to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    let hook_file = manifest.hooks.as_ref().map(|h| {
        manifest.resolve_path(plugin_root, h).display().to_string()
    });

    let mcp_config = manifest.mcp_servers.as_ref().and_then(|m| {
        let path = manifest.resolve_path(plugin_root, m);
        std::fs::read_to_string(&path).ok()
    });

    Ok(LoadedPlugin {
        manifest,
        skills,
        agent_files,
        hook_file,
        mcp_config,
    })
}

/// Scan a directory for plugins.
pub fn scan_plugins(dir: &Path) -> Vec<LoadedPlugin> {
    if !dir.exists() {
        return vec![];
    }
    let mut plugins = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                if let Ok(plugin) = load_plugin(&entry.path()) {
                    plugins.push(plugin);
                }
            }
        }
    }
    plugins
}
