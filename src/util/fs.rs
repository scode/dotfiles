use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail};

/// Expands a leading `~/` in a path to the user's home directory.
///
/// Returns an error if the path starts with `~/` but the `HOME` environment
/// variable is not set. Paths not starting with `~/` are returned unchanged.
pub fn expand_tilde(path: &str) -> Result<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        match std::env::var_os("HOME") {
            Some(home) => Ok(PathBuf::from(home).join(rest)),
            None => bail!("cannot expand ~: HOME environment variable is not set"),
        }
    } else {
        Ok(PathBuf::from(path))
    }
}

/// Lexically normalizes a path by collapsing `.` and `..` components.
///
/// This function does not access the filesystem and does not resolve symlinks.
/// It is useful when `canonicalize()` cannot be used (for example, if a path
/// does not exist yet) but you still need stable path comparisons.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            c => result.push(c),
        }
    }
    result
}

/// Computes a relative path from one directory to another path.
///
/// Given a starting directory and a target path, returns a relative path
/// that when resolved from `from_dir` would reach `to`. Both paths should
/// be absolute and canonicalized for correct results.
///
/// # Example
///
/// ```
/// use std::path::Path;
/// use dotfiles::util::fs::compute_relative_path;
///
/// let from = Path::new("/home/user/documents");
/// let to = Path::new("/home/user/dotfiles/bashrc");
/// let relative = compute_relative_path(from, to);
/// assert_eq!(relative.to_str().unwrap(), "../dotfiles/bashrc");
/// ```
pub fn compute_relative_path(from_dir: &Path, to: &Path) -> PathBuf {
    let mut from_components: Vec<_> = from_dir.components().collect();
    let mut to_components: Vec<_> = to.components().collect();

    while !from_components.is_empty()
        && !to_components.is_empty()
        && from_components[0] == to_components[0]
    {
        from_components.remove(0);
        to_components.remove(0);
    }

    let mut result = PathBuf::new();
    for _ in &from_components {
        result.push("..");
    }
    for component in to_components {
        result.push(component);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_without_home_prefix() {
        let result = expand_tilde("/absolute/path/file.txt").unwrap();
        assert_eq!(result, PathBuf::from("/absolute/path/file.txt"));
    }

    #[test]
    fn expand_tilde_with_home_set() {
        let result = expand_tilde("~/config/file.txt").unwrap();
        assert!(result.to_string_lossy().ends_with("/config/file.txt"));
        assert!(!result.to_string_lossy().starts_with("~"));
    }

    #[test]
    fn compute_relative_path_sibling_dirs() {
        let from = Path::new("/home/user/dest");
        let to = Path::new("/home/user/source/file.txt");
        let result = compute_relative_path(from, to);
        assert_eq!(result, PathBuf::from("../source/file.txt"));
    }

    #[test]
    fn compute_relative_path_nested() {
        let from = Path::new("/home/user/a/b/c");
        let to = Path::new("/home/user/x/y.txt");
        let result = compute_relative_path(from, to);
        assert_eq!(result, PathBuf::from("../../../x/y.txt"));
    }

    #[test]
    fn compute_relative_path_same_dir() {
        let from = Path::new("/home/user");
        let to = Path::new("/home/user/file.txt");
        let result = compute_relative_path(from, to);
        assert_eq!(result, PathBuf::from("file.txt"));
    }

    #[test]
    fn compute_relative_path_completely_different() {
        let from = Path::new("/a/b/c");
        let to = Path::new("/x/y/z");
        let result = compute_relative_path(from, to);
        assert_eq!(result, PathBuf::from("../../../x/y/z"));
    }

    /// Ensures lexical normalization collapses `.`/`..` in relative paths.
    #[test]
    fn normalize_path_removes_dots_and_parents_relative() {
        let path = Path::new("a/./b/../c/file.txt");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::from("a/c/file.txt"));
    }

    /// Ensures lexical normalization collapses `.`/`..` in absolute paths.
    #[test]
    fn normalize_path_removes_dots_and_parents_absolute() {
        let path = Path::new("/tmp/./a/../b/file.txt");
        let result = normalize_path(path);
        assert_eq!(result, PathBuf::from("/tmp/b/file.txt"));
    }
}
