use std::ffi::OsString;
use std::fs::{self, File, OpenOptions, Permissions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use tempfile::Builder;
use tracing::debug;

/// Expands a leading `~/` in a path to the user's home directory.
///
/// Returns an error if the path starts with `~/` but the `HOME` environment
/// variable is not set. Paths not starting with `~/` are returned unchanged.
pub fn expand_tilde(path: &str) -> Result<PathBuf> {
    expand_tilde_with_home(path, std::env::var_os("HOME"))
}

/// Expands a leading `~/` using an explicitly provided home directory.
///
/// This keeps the process environment out of tests that need to exercise the
/// missing-`HOME` path. Callers that want normal runtime behavior should use
/// [`expand_tilde`].
pub fn expand_tilde_with_home(path: &str, home: Option<OsString>) -> Result<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        match home {
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

/// Classification of a symlink target against the repository ownership
/// boundary. Produced by [`repo_target`]; see it for the exact rules.
///
/// The installer only ever repoints or deletes symlinks it owns, and ownership
/// is defined as "the target lies inside this repository checkout". This enum
/// is that judgment, with the resolved path carried along so callers can log
/// or compare it:
///
/// - `Resolved`: the target exists and canonicalizes to a path inside the
///   repository. Carries the canonical path.
/// - `Broken`: the target does not fully resolve, but both its lexical form
///   and its nearest existing ancestor land inside the repository. This is the
///   normal shape for a stale link whose source was renamed or deleted.
///   Carries the lexically normalized path.
/// - `Outside`: everything else — the link is not installer-owned and must not
///   be touched.
#[derive(Debug)]
pub enum RepoTarget {
    Resolved(PathBuf),
    Broken(PathBuf),
    Outside,
}

/// Decides whether a symlink target is owned by this repository checkout.
///
/// `target` is the link's destination resolved against the link's parent
/// directory (relative link targets mean nothing without that anchor).
/// `base_dir` is the repository root and `base_canonical` its canonicalized
/// form — callers already hold both, so this takes them instead of
/// re-canonicalizing per call.
///
/// For an existing target, `canonicalize()` answers directly. For a target
/// that fails to canonicalize (typically a broken link, but any I/O failure
/// lands here too), the lexical check alone is not enough: a path can read as
/// repository-internal while an intermediate directory symlink carries it
/// outside (`<repo>/escape/missing` where `escape` links elsewhere). So the
/// broken path must pass both the lexical prefix check and canonicalization of
/// its nearest existing ancestor. The remaining asymmetry fails closed: a link
/// written through a symlinked directory *into* the repository is classified
/// `Outside` because the lexical check runs on the unresolved text — the
/// installer then refuses to touch it rather than deleting something it
/// cannot prove it owns.
pub fn repo_target(base_dir: &Path, base_canonical: &Path, target: &Path) -> RepoTarget {
    if let Ok(canonical) = target.canonicalize() {
        if canonical.starts_with(base_canonical) {
            return RepoTarget::Resolved(canonical);
        }
        return RepoTarget::Outside;
    }

    let normalized = normalize_path(target);
    if !normalized.starts_with(normalize_path(base_dir)) {
        return RepoTarget::Outside;
    }

    let mut ancestor = target;
    while !ancestor.exists() {
        let Some(parent) = ancestor.parent() else {
            return RepoTarget::Outside;
        };
        ancestor = parent;
    }

    match ancestor.canonicalize() {
        Ok(canonical) if canonical.starts_with(base_canonical) => RepoTarget::Broken(normalized),
        _ => RepoTarget::Outside,
    }
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
    let from_components: Vec<_> = from_dir.components().collect();
    let to_components: Vec<_> = to.components().collect();

    let shared_prefix_len = from_components
        .iter()
        .zip(&to_components)
        .take_while(|(from, to)| from == to)
        .count();

    let mut result = PathBuf::new();
    for _ in &from_components[shared_prefix_len..] {
        result.push("..");
    }
    for component in &to_components[shared_prefix_len..] {
        result.push(component);
    }
    result
}

/// Resolves the directory a path's file lives in.
///
/// Exists so that `write_file_atomically` and the callers that pre-check the
/// directory's existence cannot disagree about the answer. The case that makes
/// it worth a function is the bare relative filename: `Path::parent` reports
/// `Some("")` for it, and an empty path is not a directory any syscall or
/// `is_dir()` check accepts, even though the file plainly does live somewhere —
/// the current directory.
///
/// Only a path with no parent at all — `/` — has genuinely nowhere to put a
/// sibling file, and that is the error case.
pub fn parent_directory(path: &Path) -> Result<&Path> {
    match path.parent() {
        Some(parent) if parent.as_os_str().is_empty() => Ok(Path::new(".")),
        Some(parent) => Ok(parent),
        None => bail!("destination has no parent directory: {}", path.display()),
    }
}

/// Mode requested when [`write_file_atomically`] creates a file from scratch.
///
/// This is a *request*, not the resulting mode: it is passed to `open(2)`, so
/// the kernel subtracts the process umask from it. The umask may narrow this
/// and can never widen it, which is the whole point — a user running
/// `umask 077` means it, and an installer is not the place to overrule them.
///
/// 0o644 rather than the 0o666 `File::create` requests, deliberately. Matching
/// `File::create` would hand the umask alone the decision, and a permissive one
/// (`umask 002` is the default on several distributions, `umask 000` turns up
/// in container images) would then have this helper author a group- or
/// world-*writable* file. The files it writes are shell startup files and
/// configs — things executed or trusted at login — so a writable one is a
/// direct foothold for anyone who can reach it, and nothing downstream rejects
/// it the way sshd rejects a writable `~/.ssh`. Narrowing stays entirely with
/// the user; widening is not on offer.
///
/// This applies to files created here. An existing file keeps its own mode,
/// even a wide one: that is the user's setting to hold, and preserving it is
/// not the same as choosing it.
const NEW_FILE_REQUESTED_MODE: u32 = 0o644;

/// Replaces a file's contents atomically, preserving its permission bits.
///
/// Writes a temporary file in the destination's own directory, flushes it to
/// disk, then renames it over the destination. The rename is the point: a
/// concurrent reader sees either the entire old file or the entire new one and
/// never a truncated one. That matters most for files nothing in this process
/// reads — a half-written `~/.bashrc` is not discovered until the next login,
/// long after whatever interrupted the write.
///
/// The temporary file shares the destination's directory because `rename(2)`
/// cannot cross filesystems.
///
/// Permissions: an existing regular file keeps its own permission bits; a file
/// created from scratch requests 0644 and lets the umask narrow it, so it is
/// never group- or world-writable. A destination narrower than that is narrowed
/// before any content is written, never after.
///
/// A returned error does not mean nothing happened. The replacement lands with
/// a rename, so a failure after that point — flushing the directory entry —
/// reports a durability problem, not an unwritten file. The error text says so;
/// callers that surface it must not restate it as "no change was made".
///
/// Consequences callers must accept, all inherent to replace-by-rename — the
/// destination afterwards is a different inode that only carries what is
/// copied onto it explicitly:
/// - A symlink at `path` is replaced by a regular file rather than written
///   through, so callers that care about symlinks must resolve or reject them
///   first.
/// - Hard links to the old inode stop tracking the file.
/// - Owner and group are the installing process's, not the old file's. A run
///   under `sudo` therefore leaves a root-owned file behind.
/// - Extended attributes and POSIX ACLs are dropped. Note that preserved
///   "permission bits" read an ACL's mask through the group bits, so a
///   destination carrying an ACL comes back as a plain file whose group bits
///   may grant more than the ACL did.
/// - setuid, setgid, and the sticky bit are deliberately not carried over.
///   Copying them onto a new inode owned by the installing user and group
///   would re-point them at a different principal, applied to content the
///   installer chose.
/// - Write permission is needed on the destination's *directory*, not on the
///   destination itself — the opposite of what an in-place write needs, and
///   the opposite of what someone debugging a failed write will check first.
pub fn write_file_atomically(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = parent_directory(path)?;

    // symlink_metadata rather than metadata: a symlink here must not hand us
    // the mode of whatever it points at, since the rename will not preserve
    // the link anyway. Masking with 0o777 drops the setuid/setgid/sticky bits
    // per the contract above.
    let existing_mode = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => {
            Some(metadata.permissions().mode() & 0o777)
        }
        Ok(_) => None,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(e).with_context(|| format!("cannot inspect {}", path.display()));
        }
    };

    // Builder::make_in generates the random name and retries on collision the
    // same way NamedTempFile does, but lets the mode reach open(2). That matters
    // twice over. It is what lets the umask decide a new file's mode — reading
    // the umask directly is not an option, since umask(2) is a swap, so reading
    // it means temporarily setting it, which is process-global and would race
    // anything else creating files. And it is what keeps the file from ever
    // existing wider than the destination it replaces: creating at the umask
    // default and narrowing afterwards would leave a window, however brief, in
    // which a local racer could open the file and keep a readable descriptor
    // across the later chmod.
    let mut temp = Builder::new()
        .make_in(parent, |candidate| {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(existing_mode.unwrap_or(NEW_FILE_REQUESTED_MODE))
                .open(candidate)
        })
        .with_context(|| format!("cannot create a temporary file in {}", parent.display()))?;
    // This chmod only ever widens. The open-time mode above is the destination's
    // own mode minus whatever the umask cleared, so restoring the real mode can
    // only add bits back — and the never-wider-than-the-destination property is
    // delivered entirely by that open-time mode, not by anything here. Placing
    // the call before or after the write is therefore a wash for exposure; it
    // sits here because the file should be at its final mode before anyone can
    // observe content in it at all. Worth knowing: the write still succeeds
    // afterwards even for a 0400 destination, because permission was checked
    // when the handle was opened.
    if let Some(mode) = existing_mode {
        temp.as_file()
            .set_permissions(Permissions::from_mode(mode))
            .with_context(|| {
                format!(
                    "cannot apply mode {mode:o} to the replacement for {}",
                    path.display()
                )
            })?;
    }
    // Every failure from here to the rename leaves the destination untouched,
    // and each says so rather than surfacing a bare errno the graph would print
    // with no indication of which side of the rename it came from.
    temp.write_all(contents).with_context(|| {
        format!(
            "cannot write the replacement for {} (destination unchanged)",
            path.display()
        )
    })?;
    // Same two-layer flush as the directory below, but with no tolerance for
    // an unsupported plain fsync(2): the rename has not happened yet, and
    // proceeding without any flush at all would replace the destination with
    // contents no barrier ever covered.
    sync_file(temp.as_file()).with_context(|| {
        format!(
            "cannot flush the replacement for {} (destination unchanged)",
            path.display()
        )
    })?;
    temp.persist(path)
        .with_context(|| format!("cannot replace {}", path.display()))?;

    // The rename itself also needs to reach disk. Skipping this leaves a crash
    // window where the new file was fsynced but the directory entry still
    // points at the old contents, which would make a reported install a lie.
    // The contents are already in place by now, so the context says so — a bare
    // io error here reads as "nothing was written", which would be wrong.
    let dir = File::open(parent).with_context(|| {
        format!(
            "wrote {} but could not open {} to flush its directory entry",
            path.display(),
            parent.display()
        )
    })?;
    sync_directory(&dir).with_context(|| {
        format!(
            "wrote {} but could not flush the directory entry for {}",
            path.display(),
            parent.display()
        )
    })?;
    Ok(())
}

/// Flushes a file with a plain-`fsync(2)` fallback for the strong barrier.
///
/// Two layers, because `File::sync_all` is not the same syscall everywhere. On
/// Apple targets it is `F_FULLFSYNC` with no fallback of its own — a deliberate
/// std decision — and `F_FULLFSYNC` is documented only for HFS+, FAT, UDF, and
/// APFS. On an SMB-mounted home directory it returns `ENOTSUP`; on FUSE volumes,
/// `EINVAL`. So the strong barrier is attempted first, and a plain `fsync(2)`
/// second.
///
/// What this does NOT decide is whether an unsupported fallback is tolerable.
/// If the plain `fsync(2)` also fails — including with an "unsupported" errno —
/// that error propagates untouched, because the right response depends on
/// whether the write already landed: [`sync_directory`] can shrug it off
/// post-rename, while the pre-rename temp-file flush must not.
fn sync_file(file: &File) -> std::io::Result<()> {
    match file.sync_all() {
        Ok(()) => return Ok(()),
        Err(e) if !is_unsupported(e.raw_os_error()) => return Err(e),
        Err(_) => {}
    }
    rustix::fs::fsync(file).map_err(std::io::Error::from)
}

/// Flushes a directory's entries, tolerating filesystems that cannot.
///
/// If both layers of [`sync_file`] report that the filesystem simply does not
/// offer the operation, that is a static property of the mount rather than a
/// lost write, and the write already landed via `rename(2)`. Failing here
/// would turn an unfixable environmental fact into an install that reports
/// failure forever — on a network-mounted home directory, every single run.
/// Any other error still propagates: `EIO` means the flush genuinely did not
/// happen, and this file's error contract distinguishes "not written" from
/// "written but not flushed" too carefully to blanket-ignore that.
fn sync_directory(dir: &File) -> Result<()> {
    match sync_file(dir) {
        Ok(()) => Ok(()),
        Err(e) if is_unsupported(e.raw_os_error()) => {
            debug!("directory sync unsupported on this filesystem; rename already landed");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Whether an errno means "this filesystem does not implement that", as opposed
/// to "the operation failed".
fn is_unsupported(raw: Option<i32>) -> bool {
    use rustix::io::Errno;
    raw.is_some_and(|raw| {
        raw == Errno::NOTSUP.raw_os_error()
            || raw == Errno::OPNOTSUPP.raw_os_error()
            || raw == Errno::INVAL.raw_os_error()
            || raw == Errno::NOTTY.raw_os_error()
    })
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
    fn expand_tilde_reports_missing_home() {
        let err = expand_tilde_with_home("~/config/file.txt", None).unwrap_err();
        assert!(err.to_string().contains("HOME"));
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

    /// A missing destination is created with the contents it was given.
    #[test]
    fn write_file_atomically_creates_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.txt");

        write_file_atomically(&path, b"hello\n").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "hello\n");
    }

    /// A file created from scratch lets the umask narrow, and never comes out
    /// group- or world-writable no matter how permissive the umask is.
    ///
    /// Both halves matter and neither can be asserted against a constant. The
    /// umask cannot be set from a test without mutating process-global state
    /// every other test would race, so the expectation is derived from a live
    /// `File::create` in the same directory: that file's mode is `0o666 &`
    /// the umask, and masking the write bits off it gives exactly what this
    /// helper should produce. Under `umask 077` the expectation follows the
    /// umask down to 0600; under `umask 002` the reference lands at 0664 while
    /// this must still produce 0644, which is the case a hardcoded expectation
    /// would silently pass.
    #[test]
    fn write_file_atomically_creates_without_group_or_other_write() {
        let dir = tempfile::tempdir().unwrap();
        let reference = dir.path().join("reference.txt");
        File::create(&reference).unwrap();
        let reference_mode = fs::metadata(&reference).unwrap().permissions().mode() & 0o777;
        let expected = reference_mode & !0o022;

        let path = dir.path().join("new.txt");
        write_file_atomically(&path, b"hello\n").unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, expected);
        assert_eq!(mode & 0o022, 0, "must never be group- or world-writable");
    }

    /// setuid, setgid, and the sticky bit are dropped rather than carried onto
    /// the replacement inode.
    ///
    /// The replacement is owned by the installing user and their group, so
    /// preserving setgid would silently re-point it at a different principal —
    /// and apply it to content the installer chose rather than the content the
    /// bit was set for.
    #[test]
    fn write_file_atomically_drops_setuid_bits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.txt");
        fs::write(&path, "old\n").unwrap();
        fs::set_permissions(&path, Permissions::from_mode(0o6755)).unwrap();

        write_file_atomically(&path, b"new\n").unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o7777;
        assert_eq!(mode, 0o755);
    }

    /// A symlink at the destination is replaced by a regular file, and the file
    /// it pointed at is left untouched.
    ///
    /// This is the contract that makes symlink handling a caller's problem, and
    /// the only place it can be pinned: `ManagedBlock` is the sole caller and
    /// rejects symlinked destinations before reaching here. Switching the mode
    /// probe from `symlink_metadata` to `metadata` would pass every other test
    /// while silently adopting the link target's permissions.
    #[test]
    fn write_file_atomically_replaces_a_symlink_without_following_it() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "target contents\n").unwrap();
        fs::set_permissions(&target, Permissions::from_mode(0o600)).unwrap();
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        // A live reference gives the umask-derived mode a new file should get.
        // Asserting against it is what makes the probe choice observable at
        // all: following the link would adopt the target's 0600 instead, and
        // every other assertion here holds either way.
        let reference = dir.path().join("reference.txt");
        File::create(&reference).unwrap();
        let expected = (fs::metadata(&reference).unwrap().permissions().mode() & 0o777) & !0o022;

        write_file_atomically(&link, b"new\n").unwrap();

        assert!(
            !fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&link).unwrap(), "new\n");
        assert_eq!(fs::read_to_string(&target).unwrap(), "target contents\n");
        let mode = fs::metadata(&link).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, expected, "must not adopt the link target's mode");
    }

    /// A read-only destination is still replaceable, and keeps its mode.
    ///
    /// Replace-by-rename never opens the destination, so a 0400 file is
    /// replaceable where an in-place write would fail — an implementation that
    /// reached for the destination directly would fail here. Note what this
    /// test cannot see: the narrow-before-write ordering is a property of a
    /// window that exists only mid-write, so moving the chmod back below
    /// `write_all` leaves this test green. That ordering is guarded by the
    /// comment at the call site, not by this test.
    #[test]
    fn write_file_atomically_replaces_a_read_only_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("locked.txt");
        fs::write(&path, "old\n").unwrap();
        fs::set_permissions(&path, Permissions::from_mode(0o400)).unwrap();

        write_file_atomically(&path, b"new\n").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o400);
    }

    /// Replacing an existing file must not change its permissions. Rewriting a
    /// user's file is already an intrusion; silently loosening or tightening
    /// its mode on top of that would be a security-relevant surprise.
    #[test]
    fn write_file_atomically_preserves_existing_mode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.txt");
        fs::write(&path, "old\n").unwrap();
        fs::set_permissions(&path, Permissions::from_mode(0o600)).unwrap();

        write_file_atomically(&path, b"new\n").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o7777;
        assert_eq!(mode, 0o600);
    }

    /// The temporary file must not survive the write. A helper that litters
    /// dotfiles into the user's home directory on every install would be worse
    /// than the truncation risk it exists to prevent.
    #[test]
    fn write_file_atomically_leaves_no_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("only.txt");

        write_file_atomically(&path, b"contents\n").unwrap();

        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![OsString::from("only.txt")]);
    }

    /// A missing parent directory fails instead of being created. Callers own
    /// directory creation (via `ManagedDirectory` or an explicit check), and
    /// creating one here would let a typo'd destination path succeed quietly.
    #[test]
    fn write_file_atomically_fails_without_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing-dir/file.txt");

        assert!(write_file_atomically(&path, b"contents\n").is_err());
    }

    /// An ordinary path resolves to the directory holding it.
    #[test]
    fn parent_directory_of_a_nested_path() {
        let result = parent_directory(Path::new("/home/user/.bashrc")).unwrap();
        assert_eq!(result, Path::new("/home/user"));
    }

    /// A bare relative filename resolves to the current directory rather than
    /// to the empty path `Path::parent` reports.
    ///
    /// The empty path is what makes this worth pinning: it is not a directory
    /// any caller can stat, create a file in, or `is_dir()`, so a caller that
    /// took `Some("")` at face value would reject a destination that the write
    /// path handles fine. Testing the resolution directly rather than by
    /// writing to a relative path keeps this free of `set_current_dir`, which
    /// is process-global state that would race every other test.
    #[test]
    fn parent_directory_of_a_bare_filename() {
        let result = parent_directory(Path::new("bashrc")).unwrap();
        assert_eq!(result, Path::new("."));
    }

    /// The filesystem root has no directory to hold a sibling file, which is
    /// the one case that legitimately has no answer.
    #[test]
    fn parent_directory_of_the_root_fails() {
        let err = parent_directory(Path::new("/")).unwrap_err();
        assert!(err.to_string().contains("no parent directory"), "{err}");
    }
}
