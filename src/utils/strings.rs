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

/// Unit-tests for string utility functions
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_capitalize_string() {
        assert_eq!(capitalize("iris"), "Iris");
        assert_eq!(capitalize("A"), "A");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn should_handle_pretty_path_function() {
        let path = dirs::home_dir()
            .expect("Cannot get the home directory")
            .join(".cache/iris");

        let pretty_path: String = pretty_path(&path);
        assert_eq!(pretty_path, "~/.cache/iris");
    }
}
