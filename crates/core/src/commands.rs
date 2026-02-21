use std::path::Path;

#[derive(Debug, Clone)]
pub struct CustomCommand {
    pub name: String,
    pub prompt_template: String,
    pub description: String,
}

pub fn load_commands_from_dir(project_root: &Path) -> Vec<CustomCommand> {
    let dir = project_root.join(".nyzhi").join("commands");
    let entries = match std::fs::read_dir(&dir) {
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
    template
        .replace("$ARGUMENTS", arguments)
        .trim()
        .to_string()
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
}
