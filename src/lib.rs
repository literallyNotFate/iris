///
/// Iris - Simple theme switcher for CLI programs
///
pub mod cli;
pub mod commands;
pub mod core;
pub mod models;
pub mod modules;
pub mod ui;
pub mod utils;

/// Pretty assertions for tests
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

#[cfg(test)]
pub mod test_utils;
