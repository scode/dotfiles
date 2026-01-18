//! Conditions for gating feature installation.
//!
//! Conditions are evaluated at install time to decide whether a feature should
//! be installed. This allows features to be skipped when prerequisites aren't
//! met (e.g., a parent directory doesn't exist).

use std::fmt;
use std::path::PathBuf;

use crate::util::fs::expand_tilde;

/// A condition that must be met for a feature to be installed.
///
/// The `Display` impl should describe the condition in human-readable form
/// for skip messages (e.g., "path exists: ~/.claude").
pub trait Condition: fmt::Debug + fmt::Display + Send + Sync {
    fn is_met(&self) -> bool;
}

/// Condition that checks whether a path exists (file or directory).
#[derive(Debug)]
pub struct PathExists {
    path: PathBuf,
    display_path: String,
}

impl PathExists {
    pub fn new(path: impl Into<String>) -> Self {
        let path_str = path.into();
        let expanded = expand_tilde(&path_str).unwrap_or_else(|_| PathBuf::from(&path_str));
        Self {
            path: expanded,
            display_path: path_str,
        }
    }
}

impl fmt::Display for PathExists {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "path exists: {}", self.display_path)
    }
}

impl Condition for PathExists {
    fn is_met(&self) -> bool {
        self.path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn path_exists_returns_true_when_path_exists() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_file");
        fs::write(&file_path, "content").unwrap();

        let condition = PathExists::new(file_path.to_string_lossy().to_string());
        assert!(condition.is_met());
    }

    #[test]
    fn path_exists_returns_false_when_path_missing() {
        let condition = PathExists::new("/nonexistent/path/that/does/not/exist");
        assert!(!condition.is_met());
    }

    #[test]
    fn path_exists_returns_true_for_directory() {
        let dir = tempdir().unwrap();

        let condition = PathExists::new(dir.path().to_string_lossy().to_string());
        assert!(condition.is_met());
    }

    #[test]
    fn path_exists_display_shows_original_path() {
        let condition = PathExists::new("~/.claude");
        assert_eq!(format!("{}", condition), "path exists: ~/.claude");
    }
}
