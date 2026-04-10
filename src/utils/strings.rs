/// Helper function to capitalize the first letter of string
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
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
}
