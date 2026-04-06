//! Test utilities for feature implementations.
//!
//! Provides helpers for setting up isolated test environments with temporary
//! directories for source files and symlink destinations.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// A test context providing isolated temporary directories for testing
/// file operations.
///
/// Creates two separate temporary directories:
/// - `source_dir`: For source files that features will install from
/// - `dest_dir`: For destination paths where symlinks will be created
///
/// Both directories are automatically cleaned up when the context is dropped.
///
/// # Example
///
/// ```ignore
/// let ctx = TestContext::new();
/// ctx.create_source_file("config", "contents");
/// let dest = ctx.dest_path_str("link");
/// // Test your feature...
/// ```
pub struct TestContext {
    pub source_dir: TempDir,
    pub dest_dir: TempDir,
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TestContext {
    /// Creates a new test context with fresh temporary directories.
    pub fn new() -> Self {
        Self {
            source_dir: TempDir::new().unwrap(),
            dest_dir: TempDir::new().unwrap(),
        }
    }

    /// Creates a file in the source directory with the given content.
    ///
    /// Parent directories are created automatically if needed.
    ///
    /// # Arguments
    ///
    /// * `name` - Relative path within the source directory (e.g., "config/settings")
    /// * `content` - Content to write to the file
    ///
    /// # Returns
    ///
    /// The absolute path to the created file.
    pub fn create_source_file(&self, name: &str, content: &str) -> PathBuf {
        let path = self.source_dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    /// Returns the absolute path for a file in the destination directory.
    pub fn dest_path(&self, name: &str) -> PathBuf {
        self.dest_dir.path().join(name)
    }

    /// Returns the absolute path for a file in the destination directory as a String.
    pub fn dest_path_str(&self, name: &str) -> String {
        self.dest_path(name).to_string_lossy().into_owned()
    }

    /// Returns the source directory path, used as the base directory for features.
    pub fn base_dir(&self) -> &Path {
        self.source_dir.path()
    }

    /// Returns the absolute path for a file in the source directory as a String.
    pub fn source_path_str(&self, name: &str) -> String {
        self.source_dir
            .path()
            .join(name)
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_separate_directories() {
        let ctx = TestContext::new();
        assert!(ctx.source_dir.path().exists());
        assert!(ctx.dest_dir.path().exists());
        assert_ne!(ctx.source_dir.path(), ctx.dest_dir.path());
    }

    #[test]
    fn create_source_file_simple() {
        let ctx = TestContext::new();
        let path = ctx.create_source_file("test.txt", "hello");

        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
        assert_eq!(path, ctx.source_dir.path().join("test.txt"));
    }

    #[test]
    fn create_source_file_with_subdirectory() {
        let ctx = TestContext::new();
        let path = ctx.create_source_file("path/to/file.txt", "nested content");

        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "nested content");
        assert!(ctx.source_dir.path().join("path").is_dir());
        assert!(ctx.source_dir.path().join("path/to").is_dir());
    }

    #[test]
    fn create_source_file_deeply_nested() {
        let ctx = TestContext::new();
        let path = ctx.create_source_file("a/b/c/d/e/file.txt", "deep");

        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "deep");
    }

    #[test]
    fn dest_path_simple() {
        let ctx = TestContext::new();
        let path = ctx.dest_path("link");

        assert_eq!(path, ctx.dest_dir.path().join("link"));
        assert!(!path.exists());
    }

    #[test]
    fn dest_path_with_subdirectory() {
        let ctx = TestContext::new();
        let path = ctx.dest_path("path/to/link");

        assert_eq!(path, ctx.dest_dir.path().join("path/to/link"));
    }

    #[test]
    fn dest_path_str_simple() {
        let ctx = TestContext::new();
        let path_str = ctx.dest_path_str("link");

        assert!(path_str.ends_with("/link"));
        assert!(!path_str.contains("//"));
    }

    #[test]
    fn dest_path_str_with_subdirectory() {
        let ctx = TestContext::new();
        let path_str = ctx.dest_path_str("path/to/link");

        assert!(path_str.ends_with("/path/to/link"));
    }

    #[test]
    fn base_dir_returns_source_dir() {
        let ctx = TestContext::new();
        assert_eq!(ctx.base_dir(), ctx.source_dir.path());
    }

    #[test]
    fn source_and_dest_are_independent() {
        let ctx = TestContext::new();
        ctx.create_source_file("shared_name", "source content");
        let dest = ctx.dest_path("shared_name");

        assert!(!dest.exists());
        assert!(ctx.source_dir.path().join("shared_name").exists());
    }
}
