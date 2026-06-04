///
/// Iris - Simple theme switcher for CLI programs
///
pub mod cli;
pub mod commands;
pub mod core;
pub mod guards;
pub mod log;
pub mod models;
pub mod modules;
pub mod utils;

/// Pretty assertions for tests
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
