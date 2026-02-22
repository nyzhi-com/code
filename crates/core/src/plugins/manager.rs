use std::path::Path;

use anyhow::{Context, Result};

use super::loader::{self, LoadedPlugin};
use super::{local_plugins_dir, project_plugins_dir, user_plugins_dir, PluginScope};

#[derive(Debug)]
pub struct PluginManager {
    pub plugins: Vec<(PluginScope, LoadedPlugin)>,
}

impl PluginManager {
    /// Load all enabled plugins from all scopes (local > project > user).
    pub fn load_all(project_root: &Path) -> Self {
        let mut plugins = Vec::new();

        for p in loader::scan_plugins(&local_plugins_dir(project_root)) {
            plugins.push((PluginScope::Local, p));
        }
        for p in loader::scan_plugins(&project_plugins_dir(project_root)) {
            plugins.push((PluginScope::Project, p));
        }
        for p in loader::scan_plugins(&user_plugins_dir()) {
            plugins.push((PluginScope::User, p));
        }

        Self { plugins }
    }

    /// Install a plugin from a local path.
    pub fn install_local(project_root: &Path, source: &Path, scope: PluginScope) -> Result<String> {
        let manifest = super::manifest::PluginManifest::load(source)?;
        let dest_dir = match scope {
            PluginScope::Local => local_plugins_dir(project_root),
            PluginScope::Project => project_plugins_dir(project_root),
            PluginScope::User => user_plugins_dir(),
        };

        let plugin_dir = dest_dir.join(&manifest.name);
        if plugin_dir.exists() {
            anyhow::bail!("Plugin '{}' already installed in {} scope", manifest.name, scope);
        }

        copy_dir_recursive(source, &plugin_dir)?;
        Ok(manifest.name)
    }

    /// Uninstall a plugin by name from a scope.
    pub fn uninstall(project_root: &Path, name: &str, scope: PluginScope) -> Result<()> {
        let dir = match scope {
            PluginScope::Local => local_plugins_dir(project_root),
            PluginScope::Project => project_plugins_dir(project_root),
            PluginScope::User => user_plugins_dir(),
        };

        let plugin_dir = dir.join(name);
        if plugin_dir.exists() {
            std::fs::remove_dir_all(&plugin_dir)
                .with_context(|| format!("Failed to remove plugin '{}'", name))?;
        }
        Ok(())
    }

    /// List installed plugin names with scope.
    pub fn list_installed(project_root: &Path) -> Vec<(String, PluginScope)> {
        let mut installed = Vec::new();

        for (scope, dir) in [
            (PluginScope::Local, local_plugins_dir(project_root)),
            (PluginScope::Project, project_plugins_dir(project_root)),
            (PluginScope::User, user_plugins_dir()),
        ] {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        if let Some(name) = entry.file_name().to_str() {
                            installed.push((name.to_string(), scope.clone()));
                        }
                    }
                }
            }
        }
        installed
    }

    /// Get all skills from loaded plugins.
    pub fn all_skills(&self) -> Vec<crate::skills::Skill> {
        let mut skills = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (_scope, plugin) in &self.plugins {
            for skill in &plugin.skills {
                if seen.insert(skill.name.clone()) {
                    skills.push(skill.clone());
                }
            }
        }
        skills
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
