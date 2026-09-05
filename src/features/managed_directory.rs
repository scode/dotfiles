use std::fmt;
use std::fs;
use std::io;

use anyhow::{Result, bail};
use tracing::debug;

use super::{Feature, FeatureResult};
use crate::util::fs::expand_tilde;

/// A feature that manages the existence of a directory.
///
/// On install, creates the directory if it doesn't exist. On uninstall,
/// removes it only if empty (to avoid data loss). This is useful for
/// creating parent directories that other features depend on.
///
/// Unlike symlink features, this creates an actual directory rather than
/// a link. The parent directory must already exist.
#[derive(Debug)]
pub struct ManagedDirectory {
    display_path: String,
}

impl ManagedDirectory {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            display_path: path.into(),
        }
    }
}

impl fmt::Display for ManagedDirectory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "directory: {}", self.display_path)
    }
}

impl Feature for ManagedDirectory {
    fn install(&self) -> Result<FeatureResult> {
        let path = expand_tilde(&self.display_path)?;

        if path.is_dir() {
            debug!(path = %self.display_path, "directory already exists");
            return Ok(FeatureResult::NoOp);
        }
        if path.exists() {
            bail!("path exists but is not a directory: {}", self.display_path);
        }
        fs::create_dir(&path)?;
        debug!(path = %self.display_path, "created directory");
        Ok(FeatureResult::Changed)
    }

    fn uninstall(&self) -> Result<FeatureResult> {
        let path = expand_tilde(&self.display_path)?;

        // A container that is itself a symlink was placed by the user, and
        // install deliberately followed it (see "Container Directories" in
        // SPEC.md). `remove_dir` would fail on the link with ENOTDIR whatever
        // its target holds, and deleting the link is not ours to do, so leave
        // it exactly as found.
        if path
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            debug!(path = %self.display_path, "leaving symlinked container in place");
            return Ok(FeatureResult::NoOp);
        }
        if !path.exists() {
            debug!(path = %self.display_path, "directory already removed");
            return Ok(FeatureResult::NoOp);
        }
        match fs::remove_dir(&path) {
            Ok(()) => {
                debug!(path = %self.display_path, "removed directory");
                Ok(FeatureResult::Changed)
            }
            Err(e) if e.kind() == io::ErrorKind::DirectoryNotEmpty => {
                debug!(path = %self.display_path, "leaving non-empty directory");
                Ok(FeatureResult::NoOp)
            }
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// A container that is a user-placed symlink to a directory is followed on
    /// install (the links land in the target) and left alone on uninstall.
    /// This is the "Container Directories" rule in SPEC.md; before it was
    /// written down, uninstall failed here because `rmdir` refuses a symlink,
    /// which turned a deliberately relocated skills root into a broken
    /// uninstall.
    #[test]
    fn symlinked_container_is_followed_on_install_and_left_on_uninstall() {
        let parent = tempdir().unwrap();
        let target = parent.path().join("real");
        std::fs::create_dir(&target).unwrap();
        let link = parent.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let feature = ManagedDirectory::new(link.to_string_lossy().to_string());

        assert_eq!(feature.install().unwrap(), FeatureResult::NoOp);
        assert_eq!(feature.uninstall().unwrap(), FeatureResult::NoOp);
        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
        assert!(target.is_dir());
    }

    #[test]
    fn install_creates_directory() {
        let parent = tempdir().unwrap();
        let dir_path = parent.path().join("new_dir");
        let path_str = dir_path.to_string_lossy().to_string();

        let feature = ManagedDirectory::new(path_str);

        let result = feature.install().unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
    }

    #[test]
    fn install_noop_when_exists() {
        let parent = tempdir().unwrap();
        let dir_path = parent.path().join("existing");
        fs::create_dir(&dir_path).unwrap();
        let path_str = dir_path.to_string_lossy().to_string();

        let feature = ManagedDirectory::new(path_str);

        let result = feature.install().unwrap();
        assert_eq!(result, FeatureResult::NoOp);
    }

    #[test]
    fn uninstall_removes_empty_directory() {
        let parent = tempdir().unwrap();
        let dir_path = parent.path().join("to_remove");
        fs::create_dir(&dir_path).unwrap();
        let path_str = dir_path.to_string_lossy().to_string();

        let feature = ManagedDirectory::new(path_str);

        let result = feature.uninstall().unwrap();
        assert_eq!(result, FeatureResult::Changed);
        assert!(!dir_path.exists());
    }

    #[test]
    fn uninstall_noop_when_not_exists() {
        let parent = tempdir().unwrap();
        let dir_path = parent.path().join("nonexistent");
        let path_str = dir_path.to_string_lossy().to_string();

        let feature = ManagedDirectory::new(path_str);

        let result = feature.uninstall().unwrap();
        assert_eq!(result, FeatureResult::NoOp);
    }

    #[test]
    fn uninstall_noop_when_non_empty() {
        let parent = tempdir().unwrap();
        let dir_path = parent.path().join("non_empty");
        fs::create_dir(&dir_path).unwrap();
        fs::write(dir_path.join("file.txt"), "content").unwrap();
        let path_str = dir_path.to_string_lossy().to_string();

        let feature = ManagedDirectory::new(path_str);

        let result = feature.uninstall().unwrap();
        assert_eq!(result, FeatureResult::NoOp);
        assert!(dir_path.exists());
    }

    #[test]
    fn install_fails_when_file_exists() {
        let parent = tempdir().unwrap();
        let file_path = parent.path().join("existing_file");
        fs::write(&file_path, "content").unwrap();
        let path_str = file_path.to_string_lossy().to_string();

        let feature = ManagedDirectory::new(path_str);

        let result = feature.install();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }
}
