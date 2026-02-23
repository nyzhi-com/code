use std::path::Path;

#[derive(Debug, Clone)]
pub struct CustomCommand {
    pub name: String,
    pub prompt_template: String,
    pub description: String,
}

fn scan_commands_dir(dir: &Path) -> Vec<CustomCommand> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut commands = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (description, template) = parse_command_file(&content);
        if template.is_empty() {
            continue;
        }

        commands.push(CustomCommand {
            name,
            prompt_template: template,
            description,
        });
    }
    commands
}

/// Scans `.nyzhi/commands/` then `.claude/commands/`. Nyzhi wins on name collisions.
pub fn load_commands_from_dir(project_root: &Path) -> Vec<CustomCommand> {
    let mut commands = scan_commands_dir(&project_root.join(".nyzhi").join("commands"));
    let fallback = scan_commands_dir(&project_root.join(".claude").join("commands"));

    for cmd in fallback {
        if !commands.iter().any(|c| c.name == cmd.name) {
            commands.push(cmd);
        }
    }

    commands.sort_by(|a, b| a.name.cmp(&b.name));
    commands
}

fn parse_command_file(content: &str) -> (String, String) {
    let mut lines = content.lines();
    let first = lines.next().unwrap_or("");

    if let Some(desc) = first.strip_prefix("# ") {
        let template = lines.collect::<Vec<_>>().join("\n").trim().to_string();
        (desc.trim().to_string(), template)
    } else {
        (String::new(), content.trim().to_string())
    }
}

pub fn load_commands_from_config(configs: &[nyzhi_config::CommandConfig]) -> Vec<CustomCommand> {
    configs
        .iter()
        .map(|c| CustomCommand {
            name: c.name.clone(),
            prompt_template: c.prompt.clone(),
            description: c.description.clone().unwrap_or_default(),
        })
        .collect()
}

pub fn load_all_commands(
    project_root: &Path,
    config_commands: &[nyzhi_config::CommandConfig],
) -> Vec<CustomCommand> {
    let mut dir_cmds = load_commands_from_dir(project_root);
    let config_cmds = load_commands_from_config(config_commands);

    for cc in config_cmds {
        if let Some(existing) = dir_cmds.iter_mut().find(|c| c.name == cc.name) {
            *existing = cc;
        } else {
            dir_cmds.push(cc);
        }
    }

    dir_cmds.sort_by(|a, b| a.name.cmp(&b.name));
    dir_cmds
}

pub fn expand_template(template: &str, arguments: &str) -> String {
    template.replace("$ARGUMENTS", arguments).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_with_description() {
        let content = "# Review code\nPlease review $ARGUMENTS for bugs.";
        let (desc, tmpl) = parse_command_file(content);
        assert_eq!(desc, "Review code");
        assert_eq!(tmpl, "Please review $ARGUMENTS for bugs.");
    }

    #[test]
    fn parse_without_description() {
        let content = "Please review $ARGUMENTS for bugs.";
        let (desc, tmpl) = parse_command_file(content);
        assert!(desc.is_empty());
        assert_eq!(tmpl, "Please review $ARGUMENTS for bugs.");
    }

    #[test]
    fn expand_replaces_arguments() {
        let result = expand_template("Review $ARGUMENTS carefully", "src/main.rs");
        assert_eq!(result, "Review src/main.rs carefully");
    }

    #[test]
    fn expand_no_arguments() {
        let result = expand_template("Run all tests", "");
        assert_eq!(result, "Run all tests");
    }

    #[test]
    fn dual_directory_nyzhi_wins() {
        let dir = tempfile::tempdir().unwrap();
        let nyzhi_cmds = dir.path().join(".nyzhi").join("commands");
        let claude_cmds = dir.path().join(".claude").join("commands");
        std::fs::create_dir_all(&nyzhi_cmds).unwrap();
        std::fs::create_dir_all(&claude_cmds).unwrap();
        std::fs::write(nyzhi_cmds.join("review.md"), "# Nyzhi review\nReview nyzhi").unwrap();
        std::fs::write(
            claude_cmds.join("review.md"),
            "# Claude review\nReview claude",
        )
        .unwrap();
        std::fs::write(claude_cmds.join("deploy.md"), "# Deploy\nDeploy $ARGUMENTS").unwrap();

        let cmds = load_commands_from_dir(dir.path());
        let review = cmds.iter().find(|c| c.name == "review").unwrap();
        assert_eq!(review.description, "Nyzhi review");
        assert!(cmds.iter().any(|c| c.name == "deploy"));
    }

    #[test]
    fn claude_commands_only() {
        let dir = tempfile::tempdir().unwrap();
        let claude_cmds = dir.path().join(".claude").join("commands");
        std::fs::create_dir_all(&claude_cmds).unwrap();
        std::fs::write(claude_cmds.join("test.md"), "# Run tests\nRun all tests").unwrap();

        let cmds = load_commands_from_dir(dir.path());
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name, "test");
    }
}
