use std::fmt;
use std::fs;
use std::ops::Range;
use std::path::Path;

use anyhow::{Context, Result, bail};
use tracing::debug;

use super::{Feature, FeatureResult};
use crate::util::fs::{expand_tilde, parent_directory, write_file_atomically};

/// Namespace embedded in every marker line.
///
/// "managed block" alone is far too generic for a marker that lands in files
/// shared with arbitrary other tools, and a collision would mean two installers
/// fighting over the same region. The namespace makes the claim unambiguous.
const MARKER_NAMESPACE: &str = "scode-dotfiles";

/// Trailing text on the BEGIN line, for whoever opens the file and wonders.
///
/// This is deliberately outside the part of the marker that gets matched, so
/// the wording can be reworded later without orphaning blocks already installed
/// on real machines. See [`Markers`].
const MARKER_NOTICE: &str = "- do not edit; managed by the dotfiles installer";

/// Comment syntax used for marker lines unless a block overrides it.
///
/// `#` covers shell, most config formats, and everything this repository
/// currently installs into. Like the id, a block's comment prefix is part of
/// its marker and therefore part of its identity — see [`Markers`].
const DEFAULT_COMMENT_PREFIX: &str = "#";

/// What install does when the destination file does not exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingDestination {
    /// Create the file, containing nothing but the block.
    ///
    /// Right for files whose absence is just an unconfigured machine rather
    /// than a statement — a box with no `~/.bashrc` yet is exactly when the
    /// block needs to appear.
    ///
    /// Creates the file only, never its parent directory: a destination under a
    /// directory that does not exist is an error. Directory creation belongs to
    /// `ManagedDirectory`, so a block under, say, `~/.config/foo/` needs that
    /// feature as a dependency.
    Create,
    /// Treat the absence as "not applicable here" and install nothing.
    ///
    /// Right for files that only exist when some other tool is installed,
    /// where creating one would advertise an integration that is not there.
    ///
    /// Only the destination write is skipped. The block's payload is still
    /// read and validated first, so a payload that cannot be read or that
    /// contains a marker line fails install even on a machine without the
    /// file. A broken payload is broken identically on every machine running
    /// the same commit, and a failure that deterministic should show up
    /// everywhere rather than only on machines that happen to have the
    /// destination file.
    Skip,
}

/// Where a block lands the first time it is inserted.
///
/// This only decides the initial insertion. Once the block exists, install
/// updates it where it sits; see [`ManagedBlock`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockPosition {
    /// Insert at the end of the file. The default.
    ///
    /// The right choice unless something forces otherwise: the block runs after
    /// everything the user and other tools have put in the file, which is the
    /// conventional place for a late addition and the one least likely to
    /// change how the existing content behaves.
    Append,
    /// Insert at the top of the file.
    ///
    /// For content that has to run before an early guard in the destination.
    /// The case to reason about carefully is the Debian-style `~/.bashrc`,
    /// whose `case $- in *i*) ;; *) return;; esac` returns early for
    /// *non-interactive* shells — so appended content still runs in every
    /// interactive shell, and prepending is not needed to make it work. What
    /// prepending changes is that the block also runs for non-interactive
    /// shells, which is the classic way to break `scp` and `ssh host cmd` if
    /// the block writes anything to stdout.
    ///
    /// Inserts at line zero unconditionally, which means above anything the
    /// first line was holding: a `#!` line, so an executable script stops being
    /// one, or a byte-order mark, which ends up stranded mid-file.
    Prepend,
}

/// Manages an installer-owned region of a user-owned text file, fenced by
/// comment markers.
///
/// This exists for files no tool can claim to own. Shell startup files are the
/// motivating case: package managers, language version managers, and cloud SDK
/// installers all append to `~/.bashrc`, and the user edits it too. Symlinking
/// the whole file is not available, so instead the installer stakes out a
/// clearly labelled region and touches nothing else.
///
/// Ownership contract: the owned region is the BEGIN marker line through the
/// END marker line, inclusive. Everything outside it belongs to someone else
/// and is never reordered, reindented, or rewritten, with two carve-outs that
/// insertion cannot avoid: a blank line separating the block from neighboring
/// content, and a newline supplied to a final line that lacked one, since
/// otherwise the BEGIN marker would be welded onto the end of the user's last
/// statement. Neither is taken back on uninstall, which owns only what is
/// between the markers. Install writes the body unconditionally — edits inside
/// the fence are overwritten with no way to recover them, exactly as the marker
/// says. Uninstall removes the region whether or not the body still matches
/// what install would write; the markers are themselves the ownership claim, so
/// a drifted body is still installer state. (This is a deliberate divergence
/// from `JsonManaged`, which treats an edited value as reclaimed by the user.
/// There, a managed path sits in a namespace shared with the user's own keys;
/// here the fence is unambiguous, and honoring drift would mean remembering
/// every body every past version wrote.) A destination file is never deleted,
/// even when the block was all it contained.
///
/// Position is decided once. On the first install the block is appended or
/// prepended per [`BlockPosition`]; after that, install replaces the body
/// where the block already sits. Relocating it to the end would silently
/// reorder shell initialization relative to whatever another tool appended in
/// the meantime.
///
/// The body is copied from a payload file rather than sourced from one at
/// runtime. A one-line `source ~/.config/...` shim would keep the destination
/// stable forever, but it hides the real content from the user and from every
/// other tool and agent that reads the file, and it makes a repo pull take
/// effect in every future shell with no install step in between. For a file as
/// unforgiving as `~/.bashrc`, content that only changes when the installer
/// runs is the safer trade.
///
/// Degenerate marker states — a BEGIN with no END, an END before its BEGIN, two
/// blocks with the same id — fail rather than get repaired. Guessing wrong
/// while rewriting a login shell's startup file is expensive, and "repairing" a
/// half-present block by appending a fresh one duplicates the content on every
/// subsequent install.
///
/// Assumes UTF-8, LF-terminated text. A CRLF destination survives — its lines
/// are reassembled byte for byte — but everything this writes is LF-terminated,
/// including the marker lines and the blank separator that sits outside the
/// owned region. A CRLF file therefore ends up with a few LF-only lines.
///
/// Not safe against a concurrent writer. The destination is read, edited in
/// memory, and renamed back over, so anything appended in that window is lost
/// with no error. Nothing here can fix that: advisory locking only works when
/// every writer participates, and the tools this feature exists to coexist with
/// — package hooks, SDK installers, the user's editor — do not take a lock on
/// `~/.bashrc`. The window is short and the installer is run interactively, so
/// this is an accepted limitation rather than an unsolved problem.
///
/// A failure from install does not mean the destination is untouched. The write
/// itself is atomic, but it is the last of several steps, and the one that can
/// fail after it — flushing the directory entry — leaves the new contents in
/// place. Read the error rather than assuming a rollback; there is none.
#[derive(Debug)]
pub struct ManagedBlock {
    source: String,
    destination: String,
    id: String,
    comment_prefix: String,
    position: BlockPosition,
    missing_destination: MissingDestination,
}

impl fmt::Display for ManagedBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "managed block: {} in {}", self.id, self.destination)
    }
}

impl ManagedBlock {
    /// Declares a managed block.
    ///
    /// `source` is a repository-relative path to the body, `destination`
    /// supports `~` expansion, and `id` names the block within the destination
    /// file so several independent blocks can coexist there.
    ///
    /// The id is a permanent compatibility surface. It is the only thing that
    /// identifies an already-installed block, so renaming one orphans the block
    /// on every machine that has it and needs a [`DeleteManagedBlock`] for the
    /// old id to clean up after itself.
    pub fn new(
        source: impl Into<String>,
        destination: impl Into<String>,
        id: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            destination: destination.into(),
            id: id.into(),
            comment_prefix: DEFAULT_COMMENT_PREFIX.to_owned(),
            position: BlockPosition::Append,
            missing_destination: MissingDestination::Skip,
        }
    }

    /// Sets the comment syntax used for the marker lines.
    ///
    /// Defaults to `#`. Surrounding whitespace is trimmed, so the prefix cannot
    /// make a block unmatchable against its own marker. Like the id, the prefix
    /// identifies an installed block, so changing it later needs a
    /// [`DeleteManagedBlock`] for the old spelling.
    pub fn comment_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.comment_prefix = prefix.into();
        self
    }

    /// Chooses where the block goes on first insertion.
    pub fn position(mut self, position: BlockPosition) -> Self {
        self.position = position;
        self
    }

    /// Chooses what a missing destination file means.
    ///
    /// Defaults to [`MissingDestination::Skip`], the conservative reading:
    /// authoring a file the user never had is the more surprising of the two
    /// behaviors, so it should be asked for explicitly.
    pub fn missing_destination(mut self, missing_destination: MissingDestination) -> Self {
        self.missing_destination = missing_destination;
        self
    }

    fn markers(&self) -> Result<Markers> {
        Markers::new(&self.comment_prefix, &self.id)
    }

    fn install_with_base_dir(&self, base_dir: &Path) -> Result<FeatureResult> {
        let dest_path = expand_tilde(&self.destination)?;
        let markers = self.markers()?;
        let block = markers.render(&self.read_body(base_dir)?);

        let existing = load_destination(&self.destination, &dest_path)?;
        if existing.is_none() {
            match self.missing_destination {
                MissingDestination::Skip => {
                    debug!(
                        destination = %self.destination,
                        "destination does not exist; block does not apply here"
                    );
                    return Ok(FeatureResult::NoOp);
                }
                MissingDestination::Create => {
                    // Shares parent_directory with the write path on purpose:
                    // resolving the directory two different ways is how a
                    // destination ends up rejected here and accepted there.
                    // The check itself stays, because the message it produces
                    // names the missing directory instead of surfacing a bare
                    // ENOENT from the temporary file.
                    let parent = parent_directory(&dest_path)?;
                    if !parent.is_dir() {
                        bail!(
                            "destination parent directory does not exist: {}",
                            parent.display()
                        );
                    }
                }
            }
        }

        let updated = match &existing {
            None => block,
            Some(text) => {
                let mut lines = split_lines(text);
                match locate_block(&lines, &markers, &self.destination)? {
                    Some(span) => {
                        lines.splice(span, split_lines(&block));
                    }
                    None => insert_block(&mut lines, &block, self.position),
                }
                lines.concat()
            }
        };

        if existing.as_deref() == Some(updated.as_str()) {
            debug!(destination = %self.destination, "managed block already current");
            return Ok(FeatureResult::NoOp);
        }

        write_file_atomically(&dest_path, updated.as_bytes())?;
        debug!(destination = %self.destination, id = %self.id, "wrote managed block");
        Ok(FeatureResult::Changed)
    }

    /// Reads and validates the block body.
    ///
    /// The body is returned ending with a newline unless the payload file is
    /// empty; trailing blank lines the payload actually contains are kept as
    /// authored. Supplying the missing newline is not cosmetic — without it the
    /// END marker would share a line with the body's last statement and stop
    /// being recognizable, so the block could never be found, updated, or
    /// removed again.
    ///
    /// Also rejects a body carrying marker lines; see
    /// `reject_marker_lines_in_body`.
    ///
    /// The source must resolve inside the repository, the same rule
    /// `PayloadSymlink` applies to its own source. It matters more here: a
    /// symlink is a pointer a reader can inspect, while this copies the bytes
    /// into a file on the login path. A `payload/bashrc` symlinked at anything
    /// outside the checkout — a key, a credentials file — would otherwise be
    /// inlined verbatim into the destination, and the installer would report it
    /// as an ordinary success. Git stores symlinks, so the vector is a branch
    /// or a checkout rather than a local edit.
    fn read_body(&self, base_dir: &Path) -> Result<String> {
        let source_path = base_dir.join(&self.source);
        let source_canonical = source_path
            .canonicalize()
            .with_context(|| format!("cannot resolve block body: {}", source_path.display()))?;
        let base_canonical = base_dir
            .canonicalize()
            .with_context(|| format!("cannot resolve repository root: {}", base_dir.display()))?;
        if !source_canonical.starts_with(&base_canonical) {
            bail!(
                "block body is outside repository: {} resolves to {}",
                source_path.display(),
                source_canonical.display()
            );
        }

        // Split from the read so an unreadable payload and a payload that is
        // not UTF-8 keep their own explanations; the graph renders only the
        // outermost context, so folding them together would report one as the
        // other. Same reasoning as load_destination.
        let bytes = fs::read(&source_canonical)
            .with_context(|| format!("cannot read block body: {}", source_path.display()))?;
        let mut body = String::from_utf8(bytes).with_context(|| {
            format!(
                "block body is not valid UTF-8 text: {}",
                source_path.display()
            )
        })?;
        reject_marker_lines_in_body(&body, &source_path)?;
        if !body.is_empty() && !body.ends_with('\n') {
            body.push('\n');
        }
        Ok(body)
    }
}

impl Feature for ManagedBlock {
    fn install(&self) -> Result<FeatureResult> {
        self.install_with_base_dir(&std::env::current_dir()?)
    }

    /// Removes the block, body drift and all.
    ///
    /// Deliberately independent of the payload source: uninstall must still
    /// work after the body file has been deleted from the repository, which is
    /// the usual state when a block is being retired.
    fn uninstall(&self) -> Result<FeatureResult> {
        let dest_path = expand_tilde(&self.destination)?;
        remove_block(&self.destination, &dest_path, &self.markers()?)
    }
}

/// Removes a managed block that an older installer version wrote.
///
/// The counterpart to `DeleteSymlink`, and the reason a block id or destination
/// can be changed at all: the block's own feature no longer knows the old
/// marker, so something has to. Install removes the old block; uninstall is a
/// no-op, because a deleted block cannot be restored.
#[derive(Debug)]
pub struct DeleteManagedBlock {
    destination: String,
    id: String,
    comment_prefix: String,
}

impl fmt::Display for DeleteManagedBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "delete managed block: {} in {}",
            self.id, self.destination
        )
    }
}

impl DeleteManagedBlock {
    pub fn new(destination: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            destination: destination.into(),
            id: id.into(),
            comment_prefix: DEFAULT_COMMENT_PREFIX.to_owned(),
        }
    }

    /// Sets the comment syntax the retired block's markers were written with.
    ///
    /// This has to match what the old installer version wrote, not what the
    /// current one would write, or the markers will not be found.
    pub fn comment_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.comment_prefix = prefix.into();
        self
    }
}

impl Feature for DeleteManagedBlock {
    fn install(&self) -> Result<FeatureResult> {
        let dest_path = expand_tilde(&self.destination)?;
        remove_block(
            &self.destination,
            &dest_path,
            &Markers::new(&self.comment_prefix, &self.id)?,
        )
    }

    fn uninstall(&self) -> Result<FeatureResult> {
        Ok(FeatureResult::NoOp)
    }
}

/// The marker pair for one block, and the matching rules for finding it.
///
/// Matching keys on the `<comment prefix> BEGIN managed-block(<namespace>/<id>)`
/// prefix alone, never on the whole line. That split is the compatibility
/// contract: the human-readable notice after it can be reworded in a later
/// release, while blocks already installed on real machines keep being
/// recognized. Matching the whole line instead would mean every wording tweak
/// silently orphans the old block and appends a second copy next to it.
///
/// Two invariants hold the scheme up, and a future reformat of the marker text
/// must preserve both:
///
/// - The key ends at the closing parenthesis. That is the only reason a block
///   named `bash` cannot prefix-match `managed-block(scode-dotfiles/bash-extra)`
///   and start rewriting its neighbor's region. Moving the id to the end of the
///   marker, or dropping the parentheses, would make one id swallow another;
///   `an_id_that_prefixes_another_blocks_id_does_not_match_it` is what would
///   fail, and a reformat has to keep that test meaningful rather than merely
///   passing.
/// - The configured prefix is trimmed and required to be non-empty, so the key
///   can never begin with whitespace. That is what keeps it matchable against
///   a left-trimmed file line; a key starting with a space could not match the
///   marker it had just written, and every install would append another copy
///   while uninstall found none of them.
///
/// Matching tolerates indentation but rewriting does not preserve it: an
/// indented block is found, then re-emitted at column zero. Harmless for shell
/// and most configs, but it makes this unsuitable as-is for a destination where
/// indentation carries meaning, such as YAML or Python.
#[derive(Debug)]
struct Markers {
    begin_key: String,
    end_key: String,
}

impl Markers {
    /// Builds the marker pair, rejecting inputs that would break the key
    /// invariants documented on [`Markers`].
    ///
    /// Those invariants are only as good as what gets interpolated into the
    /// key, and both failure modes are silent and unrecoverable by the tool:
    ///
    /// - A `)` in the id ends the key early, so `bash` would prefix-match a
    ///   block named `bash)x` and splice its body over that block's region —
    ///   exactly what the closing parenthesis exists to prevent.
    /// - A line break in the id or the prefix produces a key spanning two lines,
    ///   which no single line can ever match. Install would append a fresh copy
    ///   every run and uninstall would find none of them.
    /// - A `(` or `)` in the *prefix* is the same hole from the other side: a
    ///   prefix carrying marker-shaped text makes this block's emitted line
    ///   match a neighbor's key, wedging that block permanently.
    /// - An empty prefix emits a marker line that is not a comment in any
    ///   syntax. In a shell destination bash stops at the syntax error and
    ///   abandons the rest of the file — silently dropping whatever the user
    ///   had after the block. There is no destination where a bare marker line
    ///   is inert: a format without comment syntax reads it as data instead.
    ///
    /// Both fields are repository constants, so this can only fire on a bad
    /// registration — which is precisely when a loud failure beats a marker
    /// that silently does not work.
    fn new(comment_prefix: &str, id: &str) -> Result<Self> {
        let forbidden = |c: &char| matches!(c, '(' | ')') || c.is_whitespace() || c.is_control();

        if id.is_empty() {
            bail!("managed block id must not be empty");
        }
        if let Some(bad) = id.chars().find(forbidden) {
            bail!("managed block id {id:?} must not contain {bad:?}");
        }

        let comment_prefix = comment_prefix.trim();
        if comment_prefix.is_empty() {
            bail!(
                "managed block comment prefix must not be empty: a marker line has to be a comment in the destination's syntax"
            );
        }
        if let Some(bad) = comment_prefix.chars().find(|c| forbidden(c) && *c != ' ') {
            bail!("managed block comment prefix {comment_prefix:?} must not contain {bad:?}");
        }

        // No trim of the assembled key: the prefix is already trimmed and
        // non-empty, so the key cannot begin with whitespace.
        let key = |keyword: &str| format!("{comment_prefix} {}{id})", marker_infix(keyword));
        Ok(Self {
            begin_key: key("BEGIN"),
            end_key: key("END"),
        })
    }

    fn is_begin(&self, line: &str) -> bool {
        line.trim_start().starts_with(&self.begin_key)
    }

    fn is_end(&self, line: &str) -> bool {
        line.trim_start().starts_with(&self.end_key)
    }

    /// Renders the complete block.
    ///
    /// The body must end with a newline, or be empty — anything else would put
    /// the END marker on the same line as the body's last statement, hiding it
    /// from [`Markers::is_end`] and leaving the block unfindable forever.
    /// [`ManagedBlock::read_body`] is what guarantees this.
    fn render(&self, body: &str) -> String {
        format!(
            "{} {MARKER_NOTICE}\n{body}{}\n",
            self.begin_key, self.end_key
        )
    }
}

/// The part of a marker between the comment prefix and the block id.
///
/// The single definition of the key's shape. [`Markers::new`] builds keys from
/// it and [`reject_marker_lines_in_body`] scans for it, and those two must not
/// drift: if the rejection needle stopped matching what the key builder emits,
/// the net would silently catch nothing while every test stayed green, because
/// the tests compare against their own copy of the rendered marker.
fn marker_infix(keyword: &str) -> String {
    format!("{keyword} managed-block({MARKER_NAMESPACE}/")
}

/// Rejects a body containing anything that could be read as a managed-block
/// marker.
///
/// Without this check the first install succeeds and every later run — install
/// *and* uninstall, since both go through [`locate_block`] — fails on duplicate
/// markers, with no way out but hand-editing the destination on every machine
/// that ran the bad version. Failing before anything is written keeps a bad
/// payload a repository problem instead of a fleet problem. The realistic way
/// to hit this is seeding a payload file from a machine that already has a
/// block installed.
///
/// Deliberately not scoped to the block being installed. A marker belonging to
/// a *neighboring* block wedges that block just as permanently, and since
/// several blocks can share a destination, a payload seeded from a real
/// `~/.bashrc` is at least as likely to carry someone else's marker as its own.
/// For the same reason the comment prefix is ignored: a body carrying a `//`
/// marker would still be a marker once written into a file some other block
/// scans. The cost of the wide net is that a payload cannot contain the marker
/// text at all, even in prose — an acceptable trade against a failure mode with
/// no automated recovery.
fn reject_marker_lines_in_body(body: &str, source: &Path) -> Result<()> {
    let begin = marker_infix("BEGIN");
    let end = marker_infix("END");
    for (index, line) in body.lines().enumerate() {
        if line.contains(&begin) || line.contains(&end) {
            bail!(
                "block body contains a managed-block marker at {}:{}: {}",
                source.display(),
                index + 1,
                line.trim()
            );
        }
    }
    Ok(())
}

/// Reads the destination's text, or `None` when it does not exist.
///
/// Anything that is not a plain regular file is an error rather than a state
/// this returns. A symlink fails rather than being followed, matching how
/// `PayloadSymlink` and `JsonManaged` treat unexpected symlinks: writing
/// through one would edit a file at an address the installer was never given,
/// and replacing it would destroy a link the user set up on purpose.
fn load_destination(destination: &str, dest_path: &Path) -> Result<Option<String>> {
    let metadata = match dest_path.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e).with_context(|| format!("cannot inspect destination: {destination}"));
        }
    };

    if metadata.file_type().is_symlink() {
        bail!("destination is a symlink: {destination}");
    }
    if !metadata.is_file() {
        bail!("destination exists but is not a regular file: {destination}");
    }

    // Split from the read so each failure keeps its own explanation. The graph
    // renders feature errors with Display, which shows only the outermost
    // context, so folding both into one message would report a permission
    // problem as an encoding problem and drop the real cause entirely.
    let bytes =
        fs::read(dest_path).with_context(|| format!("cannot read destination: {destination}"))?;
    let text = String::from_utf8(bytes)
        .with_context(|| format!("destination is not valid UTF-8 text: {destination}"))?;
    Ok(Some(text))
}

/// Deletes the block from the destination if it is there.
///
/// Shared by `ManagedBlock::uninstall` and `DeleteManagedBlock::install`, which
/// are the same operation reached from opposite directions.
///
/// The blank separator line install may have added next to the block is left
/// behind — above it for an appended block, below it for a prepended one. It
/// sits outside the markers, and "only ever touch what is between the markers"
/// is worth more than tidiness. Reinstalling reuses that line rather than
/// adding another, so repeated install/uninstall cycles do not accumulate
/// blank lines on either arm.
fn remove_block(destination: &str, dest_path: &Path, markers: &Markers) -> Result<FeatureResult> {
    let Some(text) = load_destination(destination, dest_path)? else {
        return Ok(FeatureResult::NoOp);
    };

    let mut lines = split_lines(&text);
    let Some(span) = locate_block(&lines, markers, destination)? else {
        debug!(destination = %destination, "no managed block present to remove");
        return Ok(FeatureResult::NoOp);
    };

    lines.drain(span);
    write_file_atomically(dest_path, lines.concat().as_bytes())?;
    debug!(destination = %destination, "removed managed block");
    Ok(FeatureResult::Changed)
}

/// Splits text into lines that each keep their own trailing newline.
///
/// Keeping the newlines attached means the file can be reassembled with
/// `concat()` byte for byte, including the case of a final line without a
/// newline. Rebuilding from `lines()` would quietly append one, editing a
/// region the installer does not own.
///
/// Every element is exactly one line, and that uniformity is worth preserving
/// even where it is not strictly required. A rendered block could be spliced in
/// as a single multi-line element — nothing downstream indexes it — but then
/// the vector would hold two kinds of thing, and the `is_blank` separator
/// checks that inspect its first and last elements would only accidentally
/// still be right.
fn split_lines(text: &str) -> Vec<String> {
    text.split_inclusive('\n').map(str::to_owned).collect()
}

/// Finds the block's line range, markers included.
///
/// Every ambiguous arrangement is an error rather than a best guess. The states
/// this rejects are real: a half-deleted block from a hand edit, a duplicated
/// one from a copy-pasted config, an inverted pair from a bad merge. In each
/// case the installer cannot tell which text the user meant to keep, and
/// picking one would rewrite a login file on a guess.
fn locate_block(
    lines: &[String],
    markers: &Markers,
    destination: &str,
) -> Result<Option<Range<usize>>> {
    let begins: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| markers.is_begin(line))
        .map(|(index, _)| index)
        .collect();
    let ends: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| markers.is_end(line))
        .map(|(index, _)| index)
        .collect();

    match (begins.as_slice(), ends.as_slice()) {
        ([], []) => Ok(None),
        ([begin], [end]) if begin < end => Ok(Some(*begin..end + 1)),
        ([_], [_]) => bail!("managed block END marker precedes its BEGIN marker in {destination}"),
        ([], _) => bail!("managed block END marker has no BEGIN marker in {destination}"),
        (_, []) => bail!("managed block BEGIN marker has no END marker in {destination}"),
        _ => bail!(
            "found {} BEGIN and {} END markers for the same managed block in {destination}",
            begins.len(),
            ends.len()
        ),
    }
}

/// Inserts a block that is not in the file yet.
///
/// Separates the block from existing content with a single blank line, which is
/// deliberately left outside the owned region. Skipping the separator when the
/// neighboring line is already blank is what keeps install/uninstall cycles
/// from growing a stack of blank lines, since uninstall cannot remove a line it
/// does not own.
fn insert_block(lines: &mut Vec<String>, block: &str, position: BlockPosition) {
    let mut block_lines = split_lines(block);

    match position {
        BlockPosition::Append => {
            // A destination whose last line has no newline would otherwise get
            // the BEGIN marker welded onto the end of it.
            if let Some(last) = lines.last_mut()
                && !last.ends_with('\n')
            {
                last.push('\n');
            }
            if !is_blank(lines.last().map(String::as_str)) {
                lines.push("\n".to_owned());
            }
            lines.extend(block_lines);
        }
        BlockPosition::Prepend => {
            if !is_blank(lines.first().map(String::as_str)) {
                block_lines.push("\n".to_owned());
            }
            lines.splice(0..0, block_lines);
        }
    }
}

/// Treats a missing line as blank, so an empty file gets no separator.
fn is_blank(line: Option<&str>) -> bool {
    line.is_none_or(|line| line.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::testing::TestContext;
    use std::os::unix::fs::symlink;

    const BODY: &str = "export EDITOR=vi\n";

    /// Spells out the marker lines independently of the implementation.
    ///
    /// The duplication is the point. Marker text is a compatibility surface —
    /// a block already installed on a real machine is found by these exact
    /// bytes — so a change to how markers are rendered should have to be made
    /// twice, on purpose.
    fn rendered(body: &str) -> String {
        format!(
            "# BEGIN managed-block(scode-dotfiles/test-block) - do not edit; managed by the dotfiles installer\n{body}# END managed-block(scode-dotfiles/test-block)\n"
        )
    }

    /// Builds a block whose body lives in the context's source directory.
    fn block(ctx: &TestContext, destination: &str) -> ManagedBlock {
        ctx.create_source_file("body.sh", BODY);
        ManagedBlock::new("body.sh", ctx.dest_path_str(destination), "test-block")
    }

    fn install(ctx: &TestContext, block: &ManagedBlock) -> Result<FeatureResult> {
        block.install_with_base_dir(ctx.base_dir())
    }

    fn read(ctx: &TestContext, name: &str) -> String {
        fs::read_to_string(ctx.dest_path(name)).unwrap()
    }

    /// A machine that has never had the destination file still gets the block,
    /// which is the whole point of opting into creation.
    #[test]
    fn install_creates_missing_destination_when_asked() {
        let ctx = TestContext::new();
        let feature = block(&ctx, "bashrc").missing_destination(MissingDestination::Create);

        assert_eq!(install(&ctx, &feature).unwrap(), FeatureResult::Changed);
        assert_eq!(read(&ctx, "bashrc"), rendered(BODY));
    }

    /// The default reading of a missing destination is "this integration is not
    /// present on this machine", not "author a file the user never had".
    #[test]
    fn install_skips_missing_destination_by_default() {
        let ctx = TestContext::new();
        let feature = block(&ctx, "bashrc");

        assert_eq!(install(&ctx, &feature).unwrap(), FeatureResult::NoOp);
        assert!(!ctx.dest_path("bashrc").exists());
    }

    /// Content the user or another tool put in the file has to survive intact.
    /// This is the core promise that makes editing a shared file acceptable.
    #[test]
    fn install_appends_without_disturbing_existing_content() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("bashrc"), "export PATH=/opt/bin:$PATH\n").unwrap();
        let feature = block(&ctx, "bashrc");

        assert_eq!(install(&ctx, &feature).unwrap(), FeatureResult::Changed);
        assert_eq!(
            read(&ctx, "bashrc"),
            format!("export PATH=/opt/bin:$PATH\n\n{}", rendered(BODY))
        );
    }

    /// Prepending exists for content that must run before the destination's own
    /// early-exit logic, so the block has to land above everything else.
    #[test]
    fn install_prepends_above_existing_content() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("bashrc"), "[ -z \"$PS1\" ] && return\n").unwrap();
        let feature = block(&ctx, "bashrc").position(BlockPosition::Prepend);

        assert_eq!(install(&ctx, &feature).unwrap(), FeatureResult::Changed);
        assert_eq!(
            read(&ctx, "bashrc"),
            format!("{}\n[ -z \"$PS1\" ] && return\n", rendered(BODY))
        );
    }

    /// An existing block is updated where it sits. Relocating it to the end of
    /// the file would reorder this content against whatever another tool has
    /// appended since, which for shell startup files changes behavior.
    #[test]
    fn install_updates_block_in_place() {
        let ctx = TestContext::new();
        fs::write(
            ctx.dest_path("bashrc"),
            format!("before\n\n{}\nafter\n", rendered("old body\n")),
        )
        .unwrap();
        let feature = block(&ctx, "bashrc");

        assert_eq!(install(&ctx, &feature).unwrap(), FeatureResult::Changed);
        assert_eq!(
            read(&ctx, "bashrc"),
            format!("before\n\n{}\nafter\n", rendered(BODY))
        );
    }

    /// Reinstalling an unchanged block must not rewrite the file. A needless
    /// rewrite churns the mtime of a file other tools watch, and it would make
    /// the installer's changed/unchanged report meaningless.
    #[test]
    fn install_is_noop_when_block_is_current() {
        let ctx = TestContext::new();
        let feature = block(&ctx, "bashrc").missing_destination(MissingDestination::Create);
        install(&ctx, &feature).unwrap();

        assert_eq!(install(&ctx, &feature).unwrap(), FeatureResult::NoOp);
    }

    /// Edits inside the fence are overwritten, as the marker warns. Preserving
    /// them would mean the installed content depends on local history.
    #[test]
    fn install_overwrites_edits_inside_the_block() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("bashrc"), rendered("hand edited\n")).unwrap();
        let feature = block(&ctx, "bashrc");

        assert_eq!(install(&ctx, &feature).unwrap(), FeatureResult::Changed);
        assert_eq!(read(&ctx, "bashrc"), rendered(BODY));
    }

    /// Appending to a file whose last line has no newline must not weld the
    /// BEGIN marker onto that line, which would both hide the marker and
    /// corrupt the user's last statement.
    #[test]
    fn install_terminates_an_unterminated_final_line() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("bashrc"), "no trailing newline").unwrap();
        let feature = block(&ctx, "bashrc");

        install(&ctx, &feature).unwrap();
        assert_eq!(
            read(&ctx, "bashrc"),
            format!("no trailing newline\n\n{}", rendered(BODY))
        );
    }

    /// Install/uninstall cycles must converge. Uninstall leaves the separator
    /// blank line behind because it is not owned, so a reinstall that added
    /// another one would grow the file a line at a time, forever.
    #[test]
    fn repeated_install_uninstall_does_not_accumulate_blank_lines() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("bashrc"), "existing\n").unwrap();
        let feature = block(&ctx, "bashrc");

        install(&ctx, &feature).unwrap();
        let after_first = read(&ctx, "bashrc");
        feature.uninstall().unwrap();
        install(&ctx, &feature).unwrap();

        assert_eq!(read(&ctx, "bashrc"), after_first);
    }

    /// Uninstall removes the region and nothing else.
    #[test]
    fn uninstall_removes_block_and_leaves_the_rest() {
        let ctx = TestContext::new();
        fs::write(
            ctx.dest_path("bashrc"),
            format!("before\n\n{}\nafter\n", rendered(BODY)),
        )
        .unwrap();
        let feature = block(&ctx, "bashrc");

        assert_eq!(feature.uninstall().unwrap(), FeatureResult::Changed);
        assert_eq!(read(&ctx, "bashrc"), "before\n\n\nafter\n");
    }

    /// A body the user has edited is still installer state: the markers are the
    /// ownership claim, not the contents between them. Honoring drift here
    /// would leave orphaned blocks that no later version knows how to remove.
    #[test]
    fn uninstall_removes_a_drifted_block() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("bashrc"), rendered("user rewrote this\n")).unwrap();
        let feature = block(&ctx, "bashrc");

        assert_eq!(feature.uninstall().unwrap(), FeatureResult::Changed);
        assert_eq!(read(&ctx, "bashrc"), "");
    }

    /// Uninstall must not delete the destination, even when the block was all
    /// it held — the file itself was never the installer's to remove.
    #[test]
    fn uninstall_keeps_the_destination_file() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("bashrc"), rendered(BODY)).unwrap();

        block(&ctx, "bashrc").uninstall().unwrap();

        assert!(ctx.dest_path("bashrc").is_file());
    }

    /// Uninstalling something that was never installed is not an error, so a
    /// partially-installed machine can still be cleaned up.
    #[test]
    fn uninstall_is_noop_without_a_block() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("bashrc"), "unrelated\n").unwrap();
        let feature = block(&ctx, "bashrc");

        assert_eq!(feature.uninstall().unwrap(), FeatureResult::NoOp);
        assert_eq!(read(&ctx, "bashrc"), "unrelated\n");
    }

    /// A missing destination is nothing to clean up.
    #[test]
    fn uninstall_is_noop_without_a_destination() {
        let ctx = TestContext::new();

        assert_eq!(
            block(&ctx, "bashrc").uninstall().unwrap(),
            FeatureResult::NoOp
        );
    }

    /// A half-present block means someone edited by hand or a merge went wrong.
    /// Appending a fresh block "to fix it" would duplicate the content on every
    /// install from then on.
    #[test]
    fn install_fails_on_begin_without_end() {
        let ctx = TestContext::new();
        fs::write(
            ctx.dest_path("bashrc"),
            "# BEGIN managed-block(scode-dotfiles/test-block) - do not edit\nstray\n",
        )
        .unwrap();

        let err = install(&ctx, &block(&ctx, "bashrc")).unwrap_err();
        assert!(err.to_string().contains("no END marker"), "{err}");
    }

    /// The mirror image of a dangling BEGIN, and equally unsafe to guess at.
    #[test]
    fn install_fails_on_end_without_begin() {
        let ctx = TestContext::new();
        fs::write(
            ctx.dest_path("bashrc"),
            "stray\n# END managed-block(scode-dotfiles/test-block)\n",
        )
        .unwrap();

        let err = install(&ctx, &block(&ctx, "bashrc")).unwrap_err();
        assert!(err.to_string().contains("no BEGIN marker"), "{err}");
    }

    /// Inverted markers describe no coherent region, so there is nothing to
    /// replace and no safe way to invent one.
    #[test]
    fn install_fails_when_end_precedes_begin() {
        let ctx = TestContext::new();
        fs::write(
            ctx.dest_path("bashrc"),
            "# END managed-block(scode-dotfiles/test-block)\n# BEGIN managed-block(scode-dotfiles/test-block)\n",
        )
        .unwrap();

        let err = install(&ctx, &block(&ctx, "bashrc")).unwrap_err();
        assert!(err.to_string().contains("precedes"), "{err}");
    }

    /// Two blocks with one id usually mean a copy-pasted config. Updating one
    /// and leaving the other would install two conflicting versions of the same
    /// content, so this has to be surfaced instead.
    #[test]
    fn install_fails_on_duplicate_blocks() {
        let ctx = TestContext::new();
        fs::write(
            ctx.dest_path("bashrc"),
            format!("{}{}", rendered(BODY), rendered(BODY)),
        )
        .unwrap();

        let err = install(&ctx, &block(&ctx, "bashrc")).unwrap_err();
        assert!(err.to_string().contains("2 BEGIN"), "{err}");
    }

    /// A symlinked destination is deliberately not followed: rewriting through
    /// it would edit a file at an address the installer was never given, and
    /// replacing it would destroy a link the user set up on purpose.
    #[test]
    fn install_fails_on_symlinked_destination() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("real-bashrc"), "content\n").unwrap();
        symlink(ctx.dest_path("real-bashrc"), ctx.dest_path("bashrc")).unwrap();

        let err = install(&ctx, &block(&ctx, "bashrc")).unwrap_err();
        assert!(err.to_string().contains("symlink"), "{err}");
    }

    /// A `Skip` block with a broken payload still fails, rather than skipping
    /// quietly because its destination happens to be absent.
    ///
    /// This pins an ordering, not just an outcome: the payload is read and
    /// validated before the destination is examined. A payload fault is a fault
    /// in the repository, identical at a given commit on every machine, so
    /// reporting it only where the destination exists would make one broken
    /// checkout produce different answers across a fleet. Every other
    /// payload-fault test uses `Create`, where the destination always exists by
    /// the time it matters — so without this one, moving the read below the
    /// `Skip` return leaves the suite green.
    #[test]
    fn install_fails_on_a_broken_payload_even_when_skipping() {
        let ctx = TestContext::new();
        let feature = ManagedBlock::new("absent.sh", ctx.dest_path_str("bashrc"), "test-block");

        let err = install(&ctx, &feature).unwrap_err();
        assert!(
            err.to_string().contains("cannot resolve block body"),
            "{err}"
        );
        assert!(!ctx.dest_path("bashrc").exists());
    }

    /// A payload symlinked outside the repository is refused.
    ///
    /// Unlike `PayloadSymlink`, which installs a pointer a reader can inspect,
    /// this copies the bytes into a file on the login path — so a payload
    /// linked at a key or a credentials file would inline the secret and report
    /// an ordinary success. Git stores symlinks, so a branch can carry one.
    #[test]
    fn install_refuses_a_payload_symlinked_outside_the_repository() {
        let ctx = TestContext::new();
        let outside = ctx.dest_path("secret.txt");
        fs::write(&outside, "secret material\n").unwrap();
        symlink(&outside, ctx.source_dir.path().join("body.sh")).unwrap();
        let feature = ManagedBlock::new("body.sh", ctx.dest_path_str("bashrc"), "test-block")
            .missing_destination(MissingDestination::Create);

        let err = install(&ctx, &feature).unwrap_err();
        assert!(err.to_string().contains("outside repository"), "{err}");
        assert!(!ctx.dest_path("bashrc").exists());
    }

    /// A payload that is not valid UTF-8 fails rather than being read lossily.
    ///
    /// The destination side has the same guard for a sharper reason, but the
    /// payload side matters too: a lossy read here would install mojibake into
    /// the user's shell startup file, atomically and with no error.
    #[test]
    fn install_fails_on_a_non_utf8_payload() {
        let ctx = TestContext::new();
        fs::write(ctx.source_dir.path().join("body.sh"), b"export A=caf\xe9\n").unwrap();
        let feature = ManagedBlock::new("body.sh", ctx.dest_path_str("bashrc"), "test-block")
            .missing_destination(MissingDestination::Create);

        let err = install(&ctx, &feature).unwrap_err();
        assert!(err.to_string().contains("not valid UTF-8"), "{err}");
    }

    /// A destination that exists but is not a regular file is rejected by name.
    ///
    /// Dropping this guard would not produce a clean failure: a directory would
    /// surface whatever the read says about `EISDIR`, and a fifo or device node
    /// would block or return something that is not the file's content at all.
    #[test]
    fn install_fails_when_the_destination_is_not_a_regular_file() {
        let ctx = TestContext::new();
        fs::create_dir(ctx.dest_path("bashrc")).unwrap();

        let err = install(&ctx, &block(&ctx, "bashrc")).unwrap_err();
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    /// An id containing a closing parenthesis is rejected rather than allowed to
    /// defeat the key's terminator.
    ///
    /// `)` is what ends the key, so an id carrying one lets this block's key
    /// prefix-match a longer id's marker and splice its body over that block's
    /// region. Failing at construction turns a silent cross-block corruption
    /// into a loud registration error.
    #[test]
    fn an_id_containing_a_parenthesis_is_rejected() {
        let ctx = TestContext::new();
        ctx.create_source_file("body.sh", BODY);
        let feature = ManagedBlock::new("body.sh", ctx.dest_path_str("bashrc"), "bad)id")
            .missing_destination(MissingDestination::Create);

        let err = install(&ctx, &feature).unwrap_err();
        assert!(err.to_string().contains("must not contain"), "{err}");
    }

    /// An id containing a line break is rejected rather than producing a key no
    /// line can match.
    ///
    /// A two-line key matches nothing, so install would append a fresh copy of
    /// the block on every run and uninstall would find none of them — silent,
    /// unbounded growth of the user's file with no way back through the tool.
    #[test]
    fn an_id_containing_a_line_break_is_rejected() {
        let ctx = TestContext::new();
        ctx.create_source_file("body.sh", BODY);
        let feature = ManagedBlock::new("body.sh", ctx.dest_path_str("bashrc"), "bad\nid")
            .missing_destination(MissingDestination::Create);

        let err = install(&ctx, &feature).unwrap_err();
        assert!(err.to_string().contains("must not contain"), "{err}");
    }

    /// A comment prefix spanning lines is rejected for the same reason an id
    /// containing a line break is: the assembled key could never match a single
    /// line, so install would stack copies and uninstall would find none.
    #[test]
    fn a_multiline_comment_prefix_is_rejected() {
        let ctx = TestContext::new();
        let feature = block(&ctx, "bashrc")
            .comment_prefix("#\n#")
            .missing_destination(MissingDestination::Create);

        let err = install(&ctx, &feature).unwrap_err();
        assert!(err.to_string().contains("must not contain"), "{err}");
    }

    /// A payload file saved without a trailing newline must not put the END
    /// marker on the same line as the body's last statement.
    ///
    /// This is the failure mode that has no recovery path: an END marker that
    /// does not start its own line is invisible to `is_end`, so the block can
    /// never be located again — every later install *and* uninstall fails on a
    /// BEGIN with no END, and the user has to hand-edit the destination.
    #[test]
    fn install_terminates_a_body_without_a_trailing_newline() {
        let ctx = TestContext::new();
        ctx.create_source_file("body.sh", "export EDITOR=vi");
        let feature = ManagedBlock::new("body.sh", ctx.dest_path_str("bashrc"), "test-block")
            .missing_destination(MissingDestination::Create);

        install(&ctx, &feature).unwrap();

        assert_eq!(read(&ctx, "bashrc"), rendered(BODY));
    }

    /// A body carrying its own marker line is rejected before anything is
    /// written.
    ///
    /// Letting it through wedges the destination permanently: install writes
    /// duplicate markers once, and from then on both install and uninstall bail
    /// on the ambiguity. The realistic trigger is seeding a payload file from a
    /// machine that already has the block installed.
    #[test]
    fn install_rejects_a_body_containing_a_marker() {
        let ctx = TestContext::new();
        ctx.create_source_file("body.sh", &rendered(BODY));
        let feature = ManagedBlock::new("body.sh", ctx.dest_path_str("bashrc"), "test-block")
            .missing_destination(MissingDestination::Create);

        let err = install(&ctx, &feature).unwrap_err();
        assert!(
            err.to_string().contains("contains a managed-block marker"),
            "{err}"
        );
        assert!(
            !ctx.dest_path("bashrc").exists(),
            "nothing should be written"
        );
    }

    /// An indented block is still found rather than treated as absent.
    ///
    /// Losing the leading-whitespace tolerance would not fail loudly: the block
    /// would look absent, so install would append a second copy beside the
    /// indented one and uninstall would then refuse to touch either.
    #[test]
    fn install_updates_an_indented_block_in_place() {
        let ctx = TestContext::new();
        let indented: String = rendered("old body\n")
            .lines()
            .map(|line| format!("    {line}\n"))
            .collect();
        fs::write(ctx.dest_path("bashrc"), &indented).unwrap();
        let feature = block(&ctx, "bashrc");

        install(&ctx, &feature).unwrap();

        let contents = read(&ctx, "bashrc");
        assert_eq!(
            contents.matches("BEGIN managed-block").count(),
            1,
            "{contents}"
        );
        assert_eq!(contents, rendered(BODY));
    }

    /// The prepend arm has to converge across install/uninstall cycles too.
    ///
    /// Its separator logic is a separate branch from append's, so the guard that
    /// keeps blank lines from accumulating can be lost on one side while the
    /// other stays covered.
    #[test]
    fn repeated_prepend_cycles_do_not_accumulate_blank_lines() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("bashrc"), "existing\n").unwrap();
        let feature = block(&ctx, "bashrc").position(BlockPosition::Prepend);

        install(&ctx, &feature).unwrap();
        let after_first = read(&ctx, "bashrc");
        feature.uninstall().unwrap();
        install(&ctx, &feature).unwrap();

        assert_eq!(read(&ctx, "bashrc"), after_first);
    }

    /// Creating a file whose directory does not exist fails with a message
    /// naming the directory.
    ///
    /// Directory creation is `ManagedDirectory`'s job, and the specific message
    /// is the reason this check exists at all rather than letting the write
    /// helper report a bare `ENOENT`.
    #[test]
    fn install_fails_when_the_parent_directory_is_missing() {
        let ctx = TestContext::new();
        let feature = block(&ctx, "absent/bashrc").missing_destination(MissingDestination::Create);

        let err = install(&ctx, &feature).unwrap_err();
        assert!(
            err.to_string().contains("parent directory does not exist"),
            "{err}"
        );
    }

    /// A missing body file is a broken registration, not a reason to install an
    /// empty block over whatever the previous version installed.
    #[test]
    fn install_fails_when_the_body_source_is_missing() {
        let ctx = TestContext::new();
        let feature = ManagedBlock::new("absent.sh", ctx.dest_path_str("bashrc"), "test-block")
            .missing_destination(MissingDestination::Create);

        let err = install(&ctx, &feature).unwrap_err();
        assert!(
            err.to_string().contains("cannot resolve block body"),
            "{err}"
        );
    }

    /// Blocks are found by id, so unrelated blocks in the same file are neither
    /// updated nor removed. This is what makes several independent blocks in one
    /// destination workable.
    #[test]
    fn blocks_with_other_ids_are_left_alone() {
        let ctx = TestContext::new();
        let other = "# BEGIN managed-block(scode-dotfiles/other)\nother body\n# END managed-block(scode-dotfiles/other)\n";
        fs::write(ctx.dest_path("bashrc"), other).unwrap();
        let feature = block(&ctx, "bashrc");

        install(&ctx, &feature).unwrap();
        assert_eq!(read(&ctx, "bashrc"), format!("{other}\n{}", rendered(BODY)));

        feature.uninstall().unwrap();
        assert_eq!(read(&ctx, "bashrc"), format!("{other}\n"));
    }

    /// An id that is a strict prefix of a neighbor's id must not match it.
    ///
    /// This is the invariant the whole matching scheme rests on, and the one a
    /// future reformat of the marker text would break silently: keys are matched
    /// with `starts_with`, so only the closing parenthesis stops `test-block`
    /// from matching `test-block-extra`. Without it, install splices this
    /// block's body over the neighbor's region and uninstall deletes the
    /// neighbor outright — with every other test still green, because the
    /// existing sibling test uses an id that is not a prefix in either
    /// direction.
    #[test]
    fn an_id_that_prefixes_another_blocks_id_does_not_match_it() {
        let ctx = TestContext::new();
        let neighbor = "# BEGIN managed-block(scode-dotfiles/test-block-extra)\nneighbor body\n# END managed-block(scode-dotfiles/test-block-extra)\n";
        fs::write(ctx.dest_path("bashrc"), neighbor).unwrap();
        let feature = block(&ctx, "bashrc");

        install(&ctx, &feature).unwrap();
        assert_eq!(
            read(&ctx, "bashrc"),
            format!("{neighbor}\n{}", rendered(BODY))
        );

        feature.uninstall().unwrap();
        assert_eq!(read(&ctx, "bashrc"), format!("{neighbor}\n"));
    }

    /// An empty comment prefix is rejected rather than emitting a marker line
    /// that is not a comment.
    ///
    /// With no prefix the marker is bare text, and bash stops at the syntax
    /// error and abandons the rest of the file — so a block appended to
    /// `~/.bashrc` would silently disable everything the user had after it,
    /// including their own hardening. No destination makes a bare marker inert:
    /// a format with no comment syntax reads it as data instead. An earlier
    /// version of this test asserted the opposite, pinning the round trip for a
    /// configuration that should never have been reachable.
    #[test]
    fn an_empty_comment_prefix_is_rejected() {
        let ctx = TestContext::new();
        let feature = block(&ctx, "config")
            .comment_prefix("   ")
            .missing_destination(MissingDestination::Create);

        let err = install(&ctx, &feature).unwrap_err();
        assert!(err.to_string().contains("must not be empty"), "{err}");
        assert!(!ctx.dest_path("config").exists());
    }

    /// A comment prefix carrying marker-shaped text is rejected.
    ///
    /// It is the id's `)` hole reached from the other side of the same key: a
    /// prefix like `# BEGIN managed-block(scode-dotfiles/victim) #` makes this
    /// block's emitted line match the victim's key, and from then on that
    /// block's install and uninstall both bail on duplicate markers.
    #[test]
    fn a_comment_prefix_containing_marker_text_is_rejected() {
        let ctx = TestContext::new();
        let feature = block(&ctx, "bashrc")
            .comment_prefix("# BEGIN managed-block(scode-dotfiles/victim) #")
            .missing_destination(MissingDestination::Create);

        let err = install(&ctx, &feature).unwrap_err();
        assert!(err.to_string().contains("must not contain"), "{err}");
        assert!(!ctx.dest_path("bashrc").exists());
    }

    /// A destination that is not valid UTF-8 fails, and is left byte-for-byte
    /// intact.
    ///
    /// `~/.bashrc` can legitimately carry stray non-UTF-8 bytes from an old
    /// editor or a latin-1 comment. The regression this guards against is
    /// someone swapping the read for a lossy one to make the installer "more
    /// robust": every other test would still pass while the next install
    /// rewrites the user's file with U+FFFD in place of their bytes, atomically
    /// and with no trace of the original. The byte-equality assertion is what
    /// fails on that, not the error message.
    #[test]
    fn install_fails_on_a_non_utf8_destination_without_touching_it() {
        let ctx = TestContext::new();
        let original: &[u8] = b"# caf\xe9\n";
        fs::write(ctx.dest_path("bashrc"), original).unwrap();

        let err = install(&ctx, &block(&ctx, "bashrc")).unwrap_err();
        assert!(err.to_string().contains("UTF-8"), "{err}");
        assert_eq!(fs::read(ctx.dest_path("bashrc")).unwrap(), original);
    }

    /// Uninstall on a non-UTF-8 destination fails and leaves it byte-for-byte
    /// intact, same as install.
    ///
    /// Today uninstall shares `load_destination` with install, so the install
    /// test above covers this path transitively. This pin exists for the
    /// refactor that splits them: without it, uninstall could quietly gain a
    /// lossy read and rewrite the user's bytes on the way out while every
    /// install-side test keeps passing.
    #[test]
    fn uninstall_fails_on_a_non_utf8_destination_without_touching_it() {
        let ctx = TestContext::new();
        let original: &[u8] = b"# caf\xe9\n";
        fs::write(ctx.dest_path("bashrc"), original).unwrap();

        let err = block(&ctx, "bashrc").uninstall().unwrap_err();
        assert!(err.to_string().contains("UTF-8"), "{err}");
        assert_eq!(fs::read(ctx.dest_path("bashrc")).unwrap(), original);
    }

    /// A body carrying a *neighboring* block's marker is rejected too.
    ///
    /// Scoping the check to this block's own marker would leave the wedge it
    /// exists to prevent wide open: a payload seeded from a real `~/.bashrc`
    /// carries whatever blocks that machine had, and writing someone else's
    /// marker into the shared destination makes that block unlocatable — its
    /// install and uninstall both bail from then on, with no automated way back.
    #[test]
    fn install_rejects_a_body_containing_another_blocks_marker() {
        let ctx = TestContext::new();
        ctx.create_source_file(
            "body.sh",
            "# BEGIN managed-block(scode-dotfiles/other)\nstray\n# END managed-block(scode-dotfiles/other)\n",
        );
        let feature = ManagedBlock::new("body.sh", ctx.dest_path_str("bashrc"), "test-block")
            .missing_destination(MissingDestination::Create);

        let err = install(&ctx, &feature).unwrap_err();
        assert!(
            err.to_string().contains("contains a managed-block marker"),
            "{err}"
        );
        assert!(
            !ctx.dest_path("bashrc").exists(),
            "nothing should be written"
        );
    }

    /// Files that are not shell scripts need their own comment syntax, or the
    /// markers would be read as content by whatever consumes the file.
    #[test]
    fn custom_comment_prefix_is_used_for_markers() {
        let ctx = TestContext::new();
        let feature = block(&ctx, "config")
            .comment_prefix("//")
            .missing_destination(MissingDestination::Create);

        install(&ctx, &feature).unwrap();
        let contents = read(&ctx, "config");
        assert!(
            contents.starts_with("// BEGIN managed-block(scode-dotfiles/test-block)"),
            "{contents}"
        );
        assert!(
            contents.ends_with("// END managed-block(scode-dotfiles/test-block)\n"),
            "{contents}"
        );
    }

    /// Retiring a block id needs a feature that still knows the old marker,
    /// since the renamed block's own feature no longer does.
    #[test]
    fn delete_managed_block_removes_a_retired_block() {
        let ctx = TestContext::new();
        fs::write(
            ctx.dest_path("bashrc"),
            format!("keep\n\n{}", rendered(BODY)),
        )
        .unwrap();
        let feature = DeleteManagedBlock::new(ctx.dest_path_str("bashrc"), "test-block");

        assert_eq!(feature.install().unwrap(), FeatureResult::Changed);
        assert_eq!(read(&ctx, "bashrc"), "keep\n\n");
    }

    /// Cleanup features run on every install, including on machines that never
    /// had the old block, so absence must be ordinary rather than an error.
    #[test]
    fn delete_managed_block_is_noop_when_absent() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("bashrc"), "unrelated\n").unwrap();

        let feature = DeleteManagedBlock::new(ctx.dest_path_str("bashrc"), "test-block");
        assert_eq!(feature.install().unwrap(), FeatureResult::NoOp);
    }

    /// Deletion is one-way: uninstall cannot resurrect a block whose body the
    /// repository no longer has.
    #[test]
    fn delete_managed_block_uninstall_is_noop() {
        let ctx = TestContext::new();
        fs::write(ctx.dest_path("bashrc"), rendered(BODY)).unwrap();
        let feature = DeleteManagedBlock::new(ctx.dest_path_str("bashrc"), "test-block");

        assert_eq!(feature.uninstall().unwrap(), FeatureResult::NoOp);
        assert_eq!(read(&ctx, "bashrc"), rendered(BODY));
    }
}
