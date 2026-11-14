//! Backend of the `swhy` desktop application.
//!
//! E0 creates the crate and its dependency on `swhy-core`. The Tauri commands
//! `open_repo`, `file_at`, `query`, `run_tests` and the `tauri` dependency are
//! added at E15 (docs/decisions.md, ADR-4).

/// Version of the core the application is built on.
pub fn core_version() -> &'static str {
    swhy_core::VERSION
}

#[cfg(test)]
mod tests {
    #[test]
    fn app_is_built_on_swhy_core() {
        assert_eq!(super::core_version(), env!("CARGO_PKG_VERSION"));
    }
}
