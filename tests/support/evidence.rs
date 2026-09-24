//! JSON evidence path plumbing for the harness.

use std::path::PathBuf;

/// Where this run writes its cucumber-JSON evidence.
pub struct JsonEvidencePath(pub PathBuf);

impl JsonEvidencePath {
    /// `THIMBL_CUKE_JSON` overrides; default `target/cucumber.json`.
    pub fn from_env() -> Self {
        let path = std::env::var("THIMBL_CUKE_JSON")
            .unwrap_or_else(|_| "target/cucumber.json".to_string());
        Self(PathBuf::from(path))
    }
}
