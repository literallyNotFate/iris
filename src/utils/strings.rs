/// Helper function to capitalize the first letter of string
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// Helper function to return relative path
pub fn pretty_path(path: &std::path::Path) -> String {
    if let Ok(home) = std::env::var("HOME") {
        let path_str = path.display().to_string();
        return path_str.replace(&home, "~");
    }

    path.display().to_string()
}

/// Helper function to format bytes into readable string (B, KB, MB)
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} b", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} kb", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} mb", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Helper function to format Duration into a human-readable string
pub fn format_duration(duration: std::time::Duration) -> String {
    use colored::*;

    let millis: u128 = duration.as_millis();
    if millis < 100 {
        return "".to_string();
    }

    let text: String = if duration.as_secs() >= 1 {
        format!("{:.2}s", duration.as_secs_f32())
    } else {
        format!("{}ms", millis)
    };

    format!(" {}{}{}", "[".dimmed(), text.yellow().bold(), "]".dimmed())
}

/// Helper function to replace theme/palette block
pub fn replace_block(content: &str, marker: &str, new_block: &str) -> String {
    let start_marker: String = format!("# [iris:begin:{}]", marker);
    let end_marker: String = format!("# [iris:end:{}]", marker);
    let wrapped_block: String = format!("{}\n{}\n{}", start_marker, new_block.trim(), end_marker);

    if let Some(start_idx) = content.find(&start_marker) {
        if let Some(end_idx) = content.find(&end_marker) {
            let end_offset: usize = end_idx + end_marker.len();

            let mut result = String::new();
            result.push_str(&content[..start_idx]);
            result.push_str(&wrapped_block);
            result.push_str(&content[end_offset..]);
            return result;
        }
    }

    let mut result = content.trim_end().to_string();
    if !result.is_empty() {
        result.push_str("\n\n");
    }
    result.push_str(&wrapped_block);
    result.push('\n');

    result
}

/// Helper to remove config key (e.g., "palette = 'theme'")
pub fn remove_key(content: &str, key: &str) -> String {
    content
        .lines()
        .filter(|l| !l.trim().starts_with(&format!("{} =", key)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Helper to remove a marker line (e.g., "# iris_theme:")
pub fn remove_marker(content: &str, marker: &str) -> String {
    content
        .lines()
        .filter(|l| !l.trim().starts_with(marker))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Unit-tests for string utility functions
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn should_capitalize_string() {
        assert_eq!(capitalize("iris"), "Iris");
        assert_eq!(capitalize("A"), "A");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn should_format_size_correctly() {
        assert_eq!(format_size(500), "500 b");
        assert_eq!(format_size(1024), "1.0 kb");
        assert_eq!(format_size(1536), "1.5 kb");
        assert_eq!(format_size(1024 * 1024), "1.0 mb");
        assert_eq!(format_size(2097152), "2.0 mb");
    }

    #[test]
    fn should_handle_format_duration_correctly() {
        assert_eq!(format_duration(Duration::from_millis(50)), "");

        let ms_output = format_duration(Duration::from_millis(250));
        assert!(ms_output.contains("250ms"));

        let s_output = format_duration(Duration::from_secs_f32(1.555));
        assert!(s_output.contains("1.55s"));
    }

    #[test]
    fn should_handle_pretty_path_function() {
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".cache/iris");
            let pretty = pretty_path(&path);
            assert_eq!(pretty, "~/.cache/iris");
        }
    }

    #[test]
    fn should_replace_existing_block() {
        let content = "line 1\n# [iris:begin:fzf]\nold content\n# [iris:end:fzf]\nline 2";
        let result = replace_block(content, "fzf", "new content");
        let expected = "line 1\n# [iris:begin:fzf]\nnew content\n# [iris:end:fzf]\nline 2";
        assert_eq!(result, expected);
    }

    #[test]
    fn should_append_block_if_markers_missing() {
        let content = "line 1\nline 2";
        let result = replace_block(content, "fzf", "new content");
        let expected = "line 1\nline 2\n\n# [iris:begin:fzf]\nnew content\n# [iris:end:fzf]\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn should_remove_key_correctly() {
        let content = "theme = 'dark'\npalette = 'gruvbox'\nother = 123";
        let result = remove_key(content, "palette");
        let expected = "theme = 'dark'\nother = 123";
        assert_eq!(result, expected);
    }

    #[test]
    fn should_remove_marker_line_correctly() {
        let content = "line 1\n[iris:begin:fzf] some config\nline 2";
        let result = remove_marker(content, "[iris:begin:fzf]");
        let expected = "line 1\nline 2";
        assert_eq!(result, expected);
    }
}
