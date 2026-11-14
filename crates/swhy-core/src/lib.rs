//! Core of `swhy`: refactoring-aware history of Swift and Objective-C code.
//!
//! Layout follows docs/TZ.md, "Архитектура и стек". E0 creates the skeleton:
//! [`model`] holds the data model, the other modules are filled in by the
//! stage named in their documentation.

pub mod cache;
pub mod chains;
pub mod classify;
pub mod git;
pub mod guard;
pub mod model;
pub mod parse;
pub mod protect;
pub mod query;
pub mod response;
pub mod tests;
pub mod track;
pub mod tracker;

/// Version of the `swhy-core` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
