use crate::infra::IrisPaths;
use colored::*;
use similar::{ChangeTag, TextDiff};
use std::{fs, path::PathBuf};

/// Defines the strategy used to compute configuration differences
pub enum DiffStyle {
    /// Standard Iris marker block enclosed by begin/end tags
    Block,
    /// Key-value line injection (e.g., `theme = dracula` or `color_theme = ...`)
    InjectKey {
        key_prefix: String,
        build_ideal_line: fn(&str) -> String,
        at_top: bool,
    },
    /// Simple string line injection strictly at the top of the file (e.g., Kitty `include`)
    InjectTop {
        build_ideal_line: fn(&str) -> String,
        /// Функция или префикс для фильтрации старых строк (чтобы удалять старые инклюды)
        line_filter: fn(&str) -> bool,
    },
    /// Complex custom composite layout handled by an isolated closure
    Custom(Box<dyn Fn(&str, &str, &PathBuf, &IrisPaths) -> anyhow::Result<Option<String>>>),
}

/// A trait that provides a unified, declarative way to compute and render configuration diffs
pub trait Diffable: super::PathResolvable {
    fn config_path(&self, _paths: &IrisPaths) -> PathBuf {
        PathBuf::new()
    }

    fn ideal_content(&self, _paths: &IrisPaths, _theme: &str) -> anyhow::Result<String> {
        Ok(String::new())
    }

    fn diff_style(&self) -> DiffStyle {
        DiffStyle::Block
    }

    fn diff(&self, paths: &IrisPaths, theme: &str) -> anyhow::Result<Option<String>> {
        let config_path: PathBuf = self.config_path(paths);
        if !config_path.exists() {
            let msg = format!("  {}", "✘ Config file not found".red());
            return Ok(Some(msg));
        }

        let current_content: String = fs::read_to_string(&config_path)?;

        match self.diff_style() {
            DiffStyle::Custom(handler) => handler(&current_content, theme, &config_path, paths),

            DiffStyle::InjectKey {
                key_prefix,
                build_ideal_line,
                at_top,
            } => {
                let ideal_line: String = build_ideal_line(theme);

                if at_top {
                    let cleaned_lines: Vec<&str> = current_content
                        .lines()
                        .filter(|l| !l.trim_start().starts_with(&key_prefix))
                        .collect();

                    let target_content = if cleaned_lines.is_empty() {
                        ideal_line
                    } else {
                        format!("{}\n{}", ideal_line, cleaned_lines.join("\n"))
                    };

                    return check_and_render(&config_path, &current_content, &target_content);
                }

                if current_content.contains(&ideal_line) {
                    return Ok(None);
                }

                let target_content = if current_content
                    .lines()
                    .any(|l| l.trim_start().starts_with(&key_prefix))
                {
                    current_content
                        .lines()
                        .map(|line| {
                            if line.trim_start().starts_with(&key_prefix) {
                                ideal_line.clone()
                            } else {
                                line.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    format!("{}\n{}", current_content.trim_end(), ideal_line)
                };

                check_and_render(&config_path, &current_content, &target_content)
            }

            DiffStyle::InjectTop {
                build_ideal_line,
                line_filter,
            } => {
                let ideal_line = build_ideal_line(theme);
                let cleaned_lines: Vec<&str> = current_content
                    .lines()
                    .filter(|l| !line_filter(l))
                    .collect();

                let mut target_lines = vec![ideal_line];
                target_lines.extend(cleaned_lines.iter().map(|s| s.to_string()));
                let target_content = target_lines.join("\n");

                check_and_render(&config_path, &current_content, &target_content)
            }

            DiffStyle::Block => {
                let ideal_inner: String = self.ideal_content(paths, theme)?;
                let marker_name: &str = self.name();

                let target_content =
                    crate::utils::replace_block(&current_content, marker_name, &ideal_inner);

                check_and_render(&config_path, &current_content, &target_content)
            }
        }
    }
}

/// Checks content equation (with trim) and renders diff if needed
#[inline]
pub fn check_and_render(
    config_path: &PathBuf,
    current: &str,
    target: &str,
) -> anyhow::Result<Option<String>> {
    if current.trim() == target.trim() {
        return Ok(None);
    }

    render_diff(config_path, current, target)
}

/// Helper function to render line-by-line colored diff output using `similar`
pub fn render_diff(
    config_path: &PathBuf,
    current: &str,
    target: &str,
) -> anyhow::Result<Option<String>> {
    let diff = TextDiff::from_lines(current, target);
    let mut output_lines = Vec::new();

    println!("File: {}", crate::utils::pretty_path(config_path).magenta());

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            output_lines.push("\n".to_string());
        }

        for op in group {
            for change in diff.iter_changes(op) {
                let (sign, line_str) = match change.tag() {
                    ChangeTag::Delete => ("-".red(), format!("{}", change.value()).red()),
                    ChangeTag::Insert => ("+".green(), format!("{}", change.value()).green()),
                    ChangeTag::Equal => {
                        (" ".normal(), format!("{}", change.value()).bright_black())
                    }
                };

                let old_no = change
                    .old_index()
                    .map(|n| format!("{:4}", n + 1))
                    .unwrap_or("    ".to_string());
                let new_no = change
                    .new_index()
                    .map(|n| format!("{:4}", n + 1))
                    .unwrap_or("    ".to_string());

                let nums = format!("{} {}", old_no.dimmed(), new_no.dimmed());
                output_lines.push(format!("{} │ {}{}", nums, sign, line_str));
            }
        }
    }

    Ok(Some(output_lines.join("")))
}
