/// Represents a basic entity that can be identified by name and type
pub trait Identifiable: Send + Sync {
    /// Returns the unique name of the module/generator (e.g., "ghostty", "alacritty")
    fn name(&self) -> &'static str;

    /// Returns the category/type of the generator (e.g., Terminal, Tool)
    fn generator_type(&self) -> crate::modules::GeneratorType;

    /// Checks whether the application binary or tool is installed on the system.
    /// Default implementation checks PATH using the generator's name
    fn is_installed(&self) -> bool {
        which::which(self.name()).is_ok()
    }
}
