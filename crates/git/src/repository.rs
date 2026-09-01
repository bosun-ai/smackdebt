//! Small, synchronous Git adapter used by the project orchestration crate.
//!
//! The adapter deliberately owns no source-analysis policy.  It only turns
//! Git's structured output into values that callers can use to build a report.
//! All commands are passed as argument vectors to `git`; refs and paths are
//! never interpolated into a shell command.

use std::ffi::OsString;
use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Output, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// An error returned by a Git operation.
#[derive(Debug)]
pub enum GitError {
    /// The path is not inside a Git worktree.
    NotRepository(PathBuf),
    /// The repository has no commit history.
    EmptyHistory,
    /// A ref or object name contained an unsafe argument.
    UnsafeArgument(String),
    /// Git returned a non-zero status.
    Command {
        /// The arguments passed to Git (excluding the executable).
        args: Vec<OsString>,
        /// Exit status, when Git was able to start and exit.
        status: Option<i32>,
        /// Git's standard error, decoded lossily for diagnostics.
        stderr: String,
    },
    /// Git emitted data that did not match the requested format.
    InvalidOutput(String),
    /// A requested object does not exist.
    MissingObject(String),
    /// An I/O operation failed.
    Io(io::Error),
    /// Git output was not valid UTF-8 where text was required.
    Utf8(std::string::FromUtf8Error),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRepository(path) => write!(f, "not a Git repository: {}", path.display()),
            Self::EmptyHistory => write!(f, "repository has no commits"),
            Self::UnsafeArgument(value) => write!(f, "unsafe Git argument: {value:?}"),
            Self::Command {
                args,
                status,
                stderr,
            } => {
                write!(f, "git command {:?} failed", args)?;
                if let Some(status) = status {
                    write!(f, " with status {status}")?;
                }
                if !stderr.trim().is_empty() {
                    write!(f, ": {}", stderr.trim())?;
                }
                Ok(())
            }
            Self::InvalidOutput(message) => write!(f, "invalid Git output: {message}"),
            Self::MissingObject(object) => write!(f, "Git object is missing: {object}"),
            Self::Io(error) => error.fmt(f),
            Self::Utf8(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for GitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Utf8(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for GitError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<std::string::FromUtf8Error> for GitError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        Self::Utf8(error)
    }
}

/// A path and its relation to the requested Git ref.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ChangedPath {
    /// Path at the worktree side of the change.
    pub path: PathBuf,
    /// Path at the base side for a rename or copy.
    pub previous_path: Option<PathBuf>,
    /// The two-column porcelain status.
    pub status: ChangeStatus,
}

/// One source change between a base ref and the current worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    current_path: PathBuf,
    base_path: PathBuf,
    current_exists: bool,
    base_exists: bool,
}

impl Change {
    pub fn current_path(&self) -> &Path {
        &self.current_path
    }

    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    pub const fn current_exists(&self) -> bool {
        self.current_exists
    }

    pub const fn base_exists(&self) -> bool {
        self.base_exists
    }
}

/// The meaningful status of one changed path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Untracked,
    Ignored,
}

impl ChangeStatus {
    fn from_code(code: u8, untracked: bool) -> Result<Self, GitError> {
        if untracked || code == b'?' {
            return Ok(Self::Untracked);
        }
        Ok(match code {
            b'A' => Self::Added,
            b'M' => Self::Modified,
            b'D' => Self::Deleted,
            b'R' => Self::Renamed,
            b'C' => Self::Copied,
            b'T' => Self::TypeChanged,
            b'U' => Self::Unmerged,
            b'!' => Self::Ignored,
            other => {
                return Err(GitError::InvalidOutput(format!(
                    "unknown status code {other:?}"
                )));
            }
        })
    }
}

/// A status snapshot for the selected worktree.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct WorktreeStatus {
    pub entries: Vec<ChangedPath>,
}

/// A normalized contributor key that is intentionally opaque outside Git
/// history aggregation.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ContributorIdentity(String);

/// One file change in a non-merge commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryChange {
    path: PathBuf,
    previous_path: Option<PathBuf>,
    added_lines: Option<u32>,
    deleted_lines: Option<u32>,
}

impl HistoryChange {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn previous_path(&self) -> Option<&Path> {
        self.previous_path.as_deref()
    }
    pub const fn added_lines(&self) -> Option<u32> {
        self.added_lines
    }
    pub const fn deleted_lines(&self) -> Option<u32> {
        self.deleted_lines
    }
}

/// Compact facts for one commit. The callback receiving this value finishes
/// before the next commit is parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCommit {
    contributor: ContributorIdentity,
    timestamp: i64,
    changes: Vec<HistoryChange>,
}

impl HistoryCommit {
    pub fn contributor(&self) -> &ContributorIdentity {
        &self.contributor
    }
    /// The moment the commit landed on the analyzed history — the committer
    /// date in whole seconds since the epoch — not when it was authored.
    pub const fn timestamp(&self) -> i64 {
        self.timestamp
    }
    pub fn changes(&self) -> &[HistoryChange] {
        &self.changes
    }
}

/// Coverage facts collected while streaming history. When a window cutoff is
/// selected, the streamed set is the windowed set, so the commit count and the
/// newest and oldest timestamps describe the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryStreamSummary {
    revision: Option<String>,
    commits: u32,
    newest_timestamp: Option<i64>,
    oldest_timestamp: Option<i64>,
    shallow: bool,
}

impl HistoryStreamSummary {
    pub fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }
    pub const fn commits(&self) -> u32 {
        self.commits
    }
    pub const fn newest_timestamp(&self) -> Option<i64> {
        self.newest_timestamp
    }
    pub const fn oldest_timestamp(&self) -> Option<i64> {
        self.oldest_timestamp
    }
    pub const fn is_shallow(&self) -> bool {
        self.shallow
    }
}

/// A repository rooted at a discovered worktree.
#[derive(Clone, Debug)]
pub struct GitRepository {
    root: PathBuf,
    processes: Arc<AtomicUsize>,
}

impl GitRepository {
    /// Discover the worktree containing `path`.
    pub fn discover(path: impl AsRef<Path>) -> Result<Self, GitError> {
        let path = path.as_ref();
        let probe = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        let output = run_git_counted(
            probe,
            [
                OsString::from("rev-parse"),
                OsString::from("--show-toplevel"),
            ],
            None,
        )?;
        if !output.status.success() {
            return Err(GitError::NotRepository(path.to_path_buf()));
        }
        let root = PathBuf::from(String::from_utf8(output.stdout)?.trim());
        if root.as_os_str().is_empty() {
            return Err(GitError::NotRepository(path.to_path_buf()));
        }
        Ok(Self {
            root,
            processes: Arc::new(AtomicUsize::new(1)),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn git_processes(&self) -> usize {
        self.processes.load(Ordering::Relaxed)
    }

    /// Return the current branch's upstream/default ref when available.
    ///
    /// Detached repositories can still use a local `main` or `master` ref;
    /// absent refs return `None` instead of inventing a comparison base.
    pub fn default_ref(&self) -> Result<Option<String>, GitError> {
        if let Ok(value) = self.run_text([
            OsString::from("symbolic-ref"),
            OsString::from("--quiet"),
            OsString::from("--short"),
            OsString::from("refs/remotes/origin/HEAD"),
        ]) {
            let value = value.trim();
            if !value.is_empty() {
                return Ok(Some(value.to_owned()));
            }
        }
        for candidate in ["origin/main", "origin/master", "main", "master"] {
            if self.ref_exists(candidate)? {
                return Ok(Some(candidate.to_owned()));
            }
        }
        Ok(None)
    }

    fn ref_exists(&self, reference: &str) -> Result<bool, GitError> {
        validate_ref(reference)?;
        let args = vec![
            OsString::from("rev-parse"),
            OsString::from("--verify"),
            OsString::from("--quiet"),
            OsString::from(format!("{}^{{commit}}", reference)),
        ];
        let output = self.run_raw(args)?;
        Ok(output.status.success())
    }

    pub fn merge_base(&self, left: &str, right: &str) -> Result<String, GitError> {
        validate_ref(left)?;
        validate_ref(right)?;
        let value = self.run_text([
            OsString::from("merge-base"),
            OsString::from(left),
            OsString::from(right),
        ])?;
        let value = value.trim();
        if value.is_empty() {
            return Err(GitError::InvalidOutput(
                "merge-base returned no object".into(),
            ));
        }
        Ok(value.to_owned())
    }

    /// Read worktree status, including all non-ignored untracked files.
    fn status(&self) -> Result<WorktreeStatus, GitError> {
        let output = self.run_raw(vec![
            OsString::from("status"),
            OsString::from("--porcelain=v1"),
            OsString::from("-z"),
            OsString::from("--untracked-files=all"),
            OsString::from("--renames"),
        ])?;
        if !output.status.success() {
            return Err(command_error(&output, vec![]));
        }
        Ok(WorktreeStatus {
            entries: parse_status_z(&output.stdout)?,
        })
    }

    /// Return base-to-worktree changes. The base is passed as a ref, never a
    /// shell fragment, and `--` makes the path boundary explicit.
    fn changed_paths(&self, base: &str) -> Result<Vec<ChangedPath>, GitError> {
        validate_ref(base)?;
        let output = self.run_raw(vec![
            OsString::from("diff"),
            OsString::from("--name-status"),
            OsString::from("-z"),
            OsString::from("-M"),
            OsString::from(base),
            OsString::from("--"),
        ])?;
        if !output.status.success() {
            return Err(command_error(&output, vec![]));
        }
        parse_name_status_z(&output.stdout)
    }

    /// Returns the sorted, unique source changes visible from a base ref.
    pub fn changes_from(&self, base: &str) -> Result<Vec<Change>, GitError> {
        let mut changed = self.changed_paths(base)?;
        for entry in self.status()?.entries {
            if entry.status == ChangeStatus::Untracked
                && !changed.iter().any(|known| known.path == entry.path)
            {
                changed.push(entry);
            }
        }
        changed.sort_by(|left, right| left.path.cmp(&right.path));
        changed.dedup_by(|left, right| left.path == right.path);
        Ok(changed
            .into_iter()
            .map(|entry| {
                let current_exists = entry.status != ChangeStatus::Deleted;
                let base_exists =
                    !matches!(entry.status, ChangeStatus::Added | ChangeStatus::Untracked);
                let base_path = entry.previous_path.unwrap_or_else(|| entry.path.clone());
                Change {
                    current_path: entry.path,
                    base_path,
                    current_exists,
                    base_exists,
                }
            })
            .collect())
    }

    /// Stream non-merge history through one Git process. When `since` selects
    /// a window cutoff, the filter runs inside the Git process itself, so
    /// commits that landed before the cutoff are never streamed; both the
    /// process filter and each commit's `timestamp` describe the landed
    /// (committer) date, so the two can never disagree. Only one commit is
    /// retained by this adapter at a time.
    pub fn stream_history(
        &self,
        since: Option<i64>,
        mut accept: impl FnMut(HistoryCommit) -> Result<(), GitError>,
    ) -> Result<HistoryStreamSummary, GitError> {
        let mut args = vec![
            OsString::from("log"),
            OsString::from("--no-merges"),
            OsString::from("--numstat"),
            OsString::from("--format=%x1e%H%x00%aN%x00%aE%x00%ct%x00"),
            OsString::from("-z"),
            OsString::from("--find-renames"),
            OsString::from("--use-mailmap"),
        ];
        if let Some(cutoff) = since {
            // Git's bare `@<epoch>` date form silently falls back to "now" for
            // small or negative numbers, which would empty the stream. The
            // internal `@<epoch> <offset>` form parses every non-negative
            // cutoff, and clamping is exact: no commit landed before the epoch.
            args.push(OsString::from(format!("--since=@{} +0000", cutoff.max(0))));
        }
        args.push(OsString::from("--"));
        let mut command = self.command(args.clone());
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(GitError::Io)?;
        #[cfg(feature = "evidence-stats")]
        crate::evidence::record_process();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| GitError::InvalidOutput("history stdout was not piped".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| GitError::InvalidOutput("history stderr was not piped".into()))?;
        let mut reader = ZeroReader::new(stdout);
        let mut stderr_reader = BufReader::new(stderr);
        let shallow = self.root.join(".git/shallow").is_file();
        let mut history = HistoryParser::default();
        let read_result = reader.read_tokens(|token| history.accept(token, &mut accept));
        let status = if read_result.is_err() {
            let _ = child.kill();
            child.wait()?
        } else {
            child.wait()?
        };
        self.processes.fetch_add(1, Ordering::Relaxed);
        read_result?;
        if !status.success() {
            let mut message = String::new();
            stderr_reader.read_to_string(&mut message)?;
            if message.contains("does not have any commits")
                || message.contains("does not have any commits yet")
            {
                return Err(GitError::EmptyHistory);
            }
            return Err(GitError::Command {
                args,
                status: status.code(),
                stderr: message,
            });
        }
        history.finish(&mut accept, shallow)
    }

    /// Start one `git cat-file --batch` process. Keep this reader alive across
    /// requests to avoid one process per changed file.
    pub fn object_reader(&self, max_pending: usize) -> Result<ObjectReader, GitError> {
        ObjectReader::start(self.root.clone(), self.processes.clone(), max_pending)
    }

    fn run_text<I>(&self, args: I) -> Result<String, GitError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let output = self.run_raw(args.into_iter().collect())?;
        if !output.status.success() {
            return Err(command_error(&output, Vec::new()));
        }
        Ok(String::from_utf8(output.stdout)?)
    }

    fn run_raw(&self, args: Vec<OsString>) -> Result<Output, GitError> {
        let output = self.command(args.clone()).output()?;
        #[cfg(feature = "evidence-stats")]
        crate::evidence::record_process();
        self.processes.fetch_add(1, Ordering::Relaxed);
        Ok(output)
    }

    fn command(&self, args: Vec<OsString>) -> Command {
        let mut command = Command::new("git");
        command
            .args(args)
            .current_dir(&self.root)
            .env("LC_ALL", "C");
        command
    }
}

fn run_git_counted(
    root: &Path,
    args: impl IntoIterator<Item = OsString>,
    _counter: Option<&AtomicUsize>,
) -> Result<Output, GitError> {
    let args: Vec<OsString> = args.into_iter().collect();
    let output = Command::new("git").args(&args).current_dir(root).output()?;
    #[cfg(feature = "evidence-stats")]
    crate::evidence::record_process();
    Ok(output)
}

fn command_error(output: &Output, args: Vec<OsString>) -> GitError {
    GitError::Command {
        args,
        status: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn validate_ref(reference: &str) -> Result<(), GitError> {
    if reference.is_empty()
        || reference.starts_with('-')
        || reference
            .bytes()
            .any(|byte| byte == 0 || byte.is_ascii_control() || byte == b' ')
    {
        return Err(GitError::UnsafeArgument(reference.to_owned()));
    }
    // Let Git's own ref checker reject malformed names, while keeping command
    // arguments safe before invoking it. The adapter never treats a ref as a
    // path or as shell input.
    Ok(())
}

fn parse_status_z(bytes: &[u8]) -> Result<Vec<ChangedPath>, GitError> {
    let mut chunks = bytes.split(|byte| *byte == 0);
    let mut result = Vec::new();
    while let Some(raw) = chunks.next() {
        if raw.is_empty() {
            continue;
        }
        if raw.len() < 3 || raw[2] != b' ' {
            return Err(GitError::InvalidOutput(
                "status record lacks XY separator".into(),
            ));
        }
        let x = raw[0];
        let y = raw[1];
        let status_code = if x == b'?' || y == b'?' {
            b'?'
        } else if x == b' ' {
            y
        } else {
            x
        };
        let path = PathBuf::from(String::from_utf8(raw[3..].to_vec())?);
        let status = ChangeStatus::from_code(status_code, status_code == b'?')?;
        let previous_path = if matches!(status, ChangeStatus::Renamed | ChangeStatus::Copied) {
            let next = chunks
                .next()
                .ok_or_else(|| GitError::InvalidOutput("rename record lacks old path".into()))?;
            Some(PathBuf::from(String::from_utf8(next.to_vec())?))
        } else {
            None
        };
        result.push(ChangedPath {
            path,
            previous_path,
            status,
        });
    }
    Ok(result)
}

fn parse_name_status_z(bytes: &[u8]) -> Result<Vec<ChangedPath>, GitError> {
    let mut chunks = bytes.split(|byte| *byte == 0);
    let mut result = Vec::new();
    while let Some(raw) = chunks.next() {
        if raw.is_empty() {
            continue;
        }
        let (status_bytes, inline_path) = match raw.iter().position(|byte| *byte == b'\t') {
            Some(tab) => (&raw[..tab], Some(&raw[tab + 1..])),
            None => (raw, None),
        };
        let code = status_bytes
            .first()
            .copied()
            .ok_or_else(|| GitError::InvalidOutput("empty name-status code".into()))?;
        let status = ChangeStatus::from_code(code, false)?;
        let first = if let Some(path) = inline_path {
            PathBuf::from(String::from_utf8(path.to_vec())?)
        } else {
            let path = chunks
                .next()
                .ok_or_else(|| GitError::InvalidOutput("name-status lacks path".into()))?;
            PathBuf::from(String::from_utf8(path.to_vec())?)
        };
        let (path, previous_path) =
            if matches!(status, ChangeStatus::Renamed | ChangeStatus::Copied) {
                let second = chunks
                    .next()
                    .ok_or_else(|| GitError::InvalidOutput("rename lacks destination".into()))?;
                (
                    PathBuf::from(String::from_utf8(second.to_vec())?),
                    Some(first),
                )
            } else {
                (first, None)
            };
        result.push(ChangedPath {
            path,
            previous_path,
            status,
        });
    }
    Ok(result)
}

#[derive(Default)]
struct HistoryParser {
    header: Vec<Vec<u8>>,
    current: Option<PendingCommit>,
    pending_rename: Option<PendingRename>,
    revision: Option<String>,
    commits: u32,
    newest_timestamp: Option<i64>,
    oldest_timestamp: Option<i64>,
}

struct PendingCommit {
    contributor: ContributorIdentity,
    timestamp: i64,
    changes: Vec<HistoryChange>,
}

struct PendingRename {
    added_lines: Option<u32>,
    deleted_lines: Option<u32>,
    old_path: Option<PathBuf>,
}

impl HistoryParser {
    fn accept(
        &mut self,
        raw: &[u8],
        accept: &mut impl FnMut(HistoryCommit) -> Result<(), GitError>,
    ) -> Result<(), GitError> {
        if raw.starts_with(&[0x1e]) {
            self.flush(accept)?;
            if self.pending_rename.is_some() {
                return Err(GitError::InvalidOutput("rename record lacks a path".into()));
            }
            self.header.clear();
            self.header.push(raw[1..].to_vec());
            return Ok(());
        }
        if !self.header.is_empty() && self.header.len() < 4 {
            self.header.push(raw.to_vec());
            if self.header.len() == 4 {
                let revision = String::from_utf8(self.header[0].clone())?;
                if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(GitError::InvalidOutput(
                        "history commit id is malformed".into(),
                    ));
                }
                let timestamp = String::from_utf8(self.header[3].clone())?
                    .parse::<i64>()
                    .map_err(|_| {
                        GitError::InvalidOutput("history timestamp is malformed".into())
                    })?;
                let mut identity = self.header[1].clone();
                identity.push(0);
                identity.extend_from_slice(&self.header[2]);
                if self.revision.is_none() {
                    self.revision = Some(revision);
                    self.newest_timestamp = Some(timestamp);
                }
                self.oldest_timestamp = Some(timestamp);
                self.current = Some(PendingCommit {
                    contributor: ContributorIdentity(String::from_utf8(identity)?),
                    timestamp,
                    changes: Vec::new(),
                });
            }
            return Ok(());
        }
        let token = raw.strip_prefix(b"\n").unwrap_or(raw);
        if token.is_empty() {
            return Ok(());
        }
        let current = self.current.as_mut().ok_or_else(|| {
            GitError::InvalidOutput("history change appeared before commit header".into())
        })?;
        if let Some(mut rename) = self.pending_rename.take() {
            let path = PathBuf::from(String::from_utf8(token.to_vec())?);
            if rename.old_path.is_none() {
                rename.old_path = Some(path);
                self.pending_rename = Some(rename);
            } else {
                current.changes.push(HistoryChange {
                    path,
                    previous_path: rename.old_path,
                    added_lines: rename.added_lines,
                    deleted_lines: rename.deleted_lines,
                });
            }
            return Ok(());
        }
        let mut fields = token.splitn(3, |byte| *byte == b'\t');
        let added = parse_numstat_count(fields.next())?;
        let deleted = parse_numstat_count(fields.next())?;
        let path = fields
            .next()
            .ok_or_else(|| GitError::InvalidOutput("history numstat lacks a path".into()))?;
        if path.is_empty() {
            self.pending_rename = Some(PendingRename {
                added_lines: added,
                deleted_lines: deleted,
                old_path: None,
            });
        } else {
            current.changes.push(HistoryChange {
                path: PathBuf::from(String::from_utf8(path.to_vec())?),
                previous_path: None,
                added_lines: added,
                deleted_lines: deleted,
            });
        }
        Ok(())
    }

    fn flush(
        &mut self,
        accept: &mut impl FnMut(HistoryCommit) -> Result<(), GitError>,
    ) -> Result<(), GitError> {
        if let Some(rename) = self.pending_rename.take() {
            return Err(GitError::InvalidOutput(if rename.old_path.is_some() {
                "rename record lacks new path".into()
            } else {
                "rename record lacks old path".into()
            }));
        }
        if let Some(commit) = self.current.take() {
            self.commits += 1;
            accept(HistoryCommit {
                contributor: commit.contributor,
                timestamp: commit.timestamp,
                changes: commit.changes,
            })?;
        }
        Ok(())
    }

    fn finish(
        mut self,
        accept: &mut impl FnMut(HistoryCommit) -> Result<(), GitError>,
        shallow: bool,
    ) -> Result<HistoryStreamSummary, GitError> {
        self.flush(accept)?;
        Ok(HistoryStreamSummary {
            revision: self.revision,
            commits: self.commits,
            newest_timestamp: self.newest_timestamp,
            oldest_timestamp: self.oldest_timestamp,
            shallow,
        })
    }
}

fn parse_numstat_count(raw: Option<&[u8]>) -> Result<Option<u32>, GitError> {
    let raw = raw.ok_or_else(|| GitError::InvalidOutput("history numstat is incomplete".into()))?;
    if raw == b"-" {
        return Ok(None);
    }
    String::from_utf8(raw.to_vec())?
        .parse()
        .map(Some)
        .map_err(|_| GitError::InvalidOutput("history line count is malformed".into()))
}

/// A single long-lived `git cat-file --batch` reader.
pub struct ObjectReader {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    max_pending: usize,
    pending: usize,
    processes: Arc<AtomicUsize>,
}

impl fmt::Debug for ObjectReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BatchObjectReader")
            .field("max_pending", &self.max_pending)
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

impl ObjectReader {
    fn start(
        root: PathBuf,
        processes: Arc<AtomicUsize>,
        max_pending: usize,
    ) -> Result<Self, GitError> {
        let max_pending = max_pending.max(1);
        let mut command = Command::new("git");
        command
            .args(["cat-file", "--batch"])
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;
        #[cfg(feature = "evidence-stats")]
        crate::evidence::record_process();
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| GitError::InvalidOutput("cat-file stdin was not piped".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| GitError::InvalidOutput("cat-file stdout was not piped".into()))?;
        processes.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            max_pending,
            pending: 0,
            processes,
        })
    }

    /// Request one object and synchronously receive its bytes. `max_pending`
    /// remains part of the API so a future project producer can queue up to a
    /// measured limit without changing the process contract.
    fn read_object(&mut self, object: &str) -> Result<Vec<u8>, GitError> {
        validate_object(object)?;
        #[cfg(feature = "evidence-stats")]
        crate::evidence::record_object_read();
        if self.pending >= self.max_pending {
            self.pending = 0;
        }
        self.stdin.write_all(object.as_bytes())?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;
        self.pending += 1;
        let mut header = Vec::new();
        self.stdout.read_until(b'\n', &mut header)?;
        if header.is_empty() {
            return Err(GitError::InvalidOutput(
                "cat-file ended before header".into(),
            ));
        }
        let header = String::from_utf8(header)?;
        let fields: Vec<_> = header.trim_end_matches('\n').split(' ').collect();
        if fields.len() >= 2 && fields[1] == "missing" {
            return Err(GitError::MissingObject(object.to_owned()));
        }
        if fields.len() != 3 {
            return Err(GitError::InvalidOutput(format!(
                "invalid cat-file header: {header:?}"
            )));
        }
        let size: usize = fields[2]
            .parse()
            .map_err(|_| GitError::InvalidOutput("invalid cat-file object size".into()))?;
        let mut bytes = vec![0; size];
        self.stdout.read_exact(&mut bytes)?;
        let mut newline = [0u8; 1];
        self.stdout.read_exact(&mut newline)?;
        if newline[0] != b'\n' {
            return Err(GitError::InvalidOutput(
                "cat-file object lacked separator".into(),
            ));
        }
        Ok(bytes)
    }

    /// Read a path from a commit without spawning another Git process.
    pub fn read_path(&mut self, reference: &str, path: &Path) -> Result<Vec<u8>, GitError> {
        validate_ref(reference)?;
        validate_path(path)?;
        let path = path
            .to_str()
            .ok_or_else(|| GitError::UnsafeArgument(path.to_string_lossy().into_owned()))?;
        self.read_object(&format!("{reference}:{path}"))
    }

    /// Lists regular files in one commit tree through this batch process.
    pub fn tree_files(&mut self, reference: &str) -> Result<Vec<PathBuf>, GitError> {
        validate_ref(reference)?;
        let root = self.read_object(&format!("{reference}^{{tree}}"))?;
        let mut files = Vec::new();
        self.collect_tree_files(&root, Path::new(""), &mut files)?;
        files.sort();
        Ok(files)
    }

    fn collect_tree_files(
        &mut self,
        tree: &[u8],
        parent: &Path,
        files: &mut Vec<PathBuf>,
    ) -> Result<(), GitError> {
        let mut offset = 0;
        while offset < tree.len() {
            let mode_end = tree[offset..]
                .iter()
                .position(|byte| *byte == b' ')
                .map(|position| offset + position)
                .ok_or_else(|| GitError::InvalidOutput("tree entry lacks mode".into()))?;
            let name_start = mode_end + 1;
            let name_end = tree[name_start..]
                .iter()
                .position(|byte| *byte == 0)
                .map(|position| name_start + position)
                .ok_or_else(|| GitError::InvalidOutput("tree entry lacks name".into()))?;
            let oid_start = name_end + 1;
            let oid_end = oid_start + 20;
            let oid = tree
                .get(oid_start..oid_end)
                .ok_or_else(|| GitError::InvalidOutput("tree entry lacks object id".into()))?;
            let name = std::str::from_utf8(&tree[name_start..name_end])
                .map_err(|_| GitError::InvalidOutput("tree path is not UTF-8".into()))?;
            let path = parent.join(name);
            let mode = &tree[offset..mode_end];
            if mode == b"40000" {
                let child = self.read_object(&hex_object_id(oid))?;
                self.collect_tree_files(&child, &path, files)?;
            } else if mode == b"100644" || mode == b"100755" {
                files.push(path);
            }
            offset = oid_end;
        }
        Ok(())
    }
}

fn hex_object_id(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}

impl Drop for ObjectReader {
    fn drop(&mut self) {
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = &self.processes;
    }
}

fn validate_object(object: &str) -> Result<(), GitError> {
    if object.is_empty() || object.bytes().any(|byte| matches!(byte, 0 | b'\n' | b'\r')) {
        Err(GitError::UnsafeArgument(object.to_owned()))
    } else {
        Ok(())
    }
}

fn validate_path(path: &Path) -> Result<(), GitError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
        || path
            .as_os_str()
            .to_string_lossy()
            .bytes()
            .any(|byte| matches!(byte, 0 | b'\n' | b'\r'))
    {
        return Err(GitError::UnsafeArgument(
            path.to_string_lossy().into_owned(),
        ));
    }
    Ok(())
}

struct ZeroReader<R> {
    reader: R,
    buffer: Vec<u8>,
}

impl<R: Read> ZeroReader<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: vec![0; 16 * 1024],
        }
    }
    fn read_tokens(
        &mut self,
        mut callback: impl FnMut(&[u8]) -> Result<(), GitError>,
    ) -> Result<(), GitError> {
        let mut token = Vec::new();
        loop {
            let size = self.reader.read(&mut self.buffer)?;
            if size == 0 {
                if !token.is_empty() {
                    callback(&token)?;
                }
                return Ok(());
            }
            for byte in &self.buffer[..size] {
                if *byte == 0 {
                    callback(&token)?;
                    token.clear();
                } else {
                    token.push(*byte);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Repo {
        path: PathBuf,
    }
    impl Repo {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "smackdebt-git-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            git(&path, ["init", "-q"]);
            git(&path, ["config", "user.email", "test@example.com"]);
            git(&path, ["config", "user.name", "Test"]);
            Self { path }
        }
        fn commit(&self, path: &str, body: &str, message: &str) {
            let full = self.path.join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            File::create(full)
                .unwrap()
                .write_all(body.as_bytes())
                .unwrap();
            git(&self.path, ["add", "."]);
            git(&self.path, ["commit", "-qm", message]);
        }
        fn commit_dated(
            &self,
            path: &str,
            body: &str,
            message: &str,
            authored: &str,
            landed: &str,
        ) {
            let full = self.path.join(path);
            File::create(full)
                .unwrap()
                .write_all(body.as_bytes())
                .unwrap();
            git(&self.path, ["add", "."]);
            let output = Command::new("git")
                .args(["commit", "-qm", message])
                .env("GIT_AUTHOR_DATE", authored)
                .env("GIT_COMMITTER_DATE", landed)
                .current_dir(&self.path)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn git<const N: usize>(cwd: &Path, args: [&str; N]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn discovers_root_and_default_branch() {
        let repo = Repo::new();
        repo.commit("main.rs", "fn main() {}", "initial");
        let adapter = GitRepository::discover(repo.path.join("main.rs")).unwrap();
        assert!(adapter.root().join("main.rs").exists());
        assert_eq!(adapter.default_ref().unwrap().as_deref(), Some("master"));
    }

    #[test]
    fn repository_without_a_default_ref_reports_none() {
        let repo = Repo::new();
        let adapter = GitRepository::discover(&repo.path).unwrap();
        assert_eq!(adapter.default_ref().unwrap(), None);
    }

    #[test]
    fn empty_repository_has_unavailable_history_from_the_history_process() {
        let repo = Repo::new();
        let adapter = GitRepository::discover(&repo.path).unwrap();
        let before = adapter.git_processes();
        assert!(matches!(
            adapter.stream_history(None, |_| Ok(())),
            Err(GitError::EmptyHistory)
        ));
        assert_eq!(adapter.git_processes() - before, 1);
    }

    #[test]
    fn status_includes_untracked_and_rename() {
        let repo = Repo::new();
        repo.commit("old.rs", "one", "initial");
        fs::rename(repo.path.join("old.rs"), repo.path.join("new.rs")).unwrap();
        git(&repo.path, ["add", "-A"]);
        File::create(repo.path.join("scratch.rs")).unwrap();
        let adapter = GitRepository::discover(&repo.path).unwrap();
        let status = adapter.status().unwrap();
        assert!(
            status
                .entries
                .iter()
                .any(|entry| entry.path == Path::new("scratch.rs")
                    && entry.status == ChangeStatus::Untracked)
        );
        assert!(
            status
                .entries
                .iter()
                .any(|entry| entry.path == Path::new("new.rs")
                    && entry.status == ChangeStatus::Renamed),
            "entries: {:?}",
            status.entries
        );
    }

    #[test]
    fn changed_paths_parses_nul_delimited_statuses_and_renames() {
        let repo = Repo::new();
        repo.commit("old.rs", "one", "initial");
        fs::rename(repo.path.join("old.rs"), repo.path.join("new.rs")).unwrap();
        git(&repo.path, ["add", "-A"]);
        let adapter = GitRepository::discover(&repo.path).unwrap();
        let changes = adapter.changed_paths("HEAD").unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].status, ChangeStatus::Renamed);
        assert_eq!(changes[0].path, Path::new("new.rs"));
        assert_eq!(
            changes[0].previous_path.as_deref(),
            Some(Path::new("old.rs"))
        );
    }

    #[test]
    fn history_uses_one_process_and_streams_churn() {
        let repo = Repo::new();
        repo.commit("a.rs", "one", "one");
        repo.commit("a.rs", "two\nthree", "two");
        let adapter = GitRepository::discover(&repo.path).unwrap();
        let before = adapter.git_processes();
        let mut commits = Vec::new();
        let summary = adapter
            .stream_history(None, |commit| {
                commits.push(commit);
                Ok(())
            })
            .unwrap();
        assert_eq!(adapter.git_processes() - before, 1);
        assert_eq!(summary.commits(), 2);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].changes()[0].path(), Path::new("a.rs"));
        assert_eq!(commits[0].changes()[0].added_lines(), Some(2));
        assert_eq!(commits[0].changes()[0].deleted_lines(), Some(1));
    }

    #[test]
    fn a_history_cutoff_filters_inside_the_git_process_on_landed_dates() {
        let repo = Repo::new();
        repo.commit_dated(
            "a.rs",
            "one",
            "old",
            "2000-01-02T03:04:05Z",
            "2000-01-02T03:04:05Z",
        );
        repo.commit_dated(
            "a.rs",
            "two",
            "recent",
            "2020-01-02T03:04:05Z",
            "2020-01-02T03:04:05Z",
        );
        let adapter = GitRepository::discover(&repo.path).unwrap();
        let mut unbounded = 0u32;
        let summary = adapter
            .stream_history(None, |_| {
                unbounded += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!((summary.commits(), unbounded), (2, 2));
        let cutoff = 1_500_000_000;
        let mut windowed = Vec::new();
        let summary = adapter
            .stream_history(Some(cutoff), |commit| {
                windowed.push(commit);
                Ok(())
            })
            .unwrap();
        // The out-of-window commit was never streamed, and the summary
        // describes the windowed set.
        assert_eq!((summary.commits(), windowed.len()), (1, 1));
        assert!(windowed[0].timestamp() >= cutoff);
        assert_eq!(summary.newest_timestamp(), Some(windowed[0].timestamp()));
        assert_eq!(summary.oldest_timestamp(), Some(windowed[0].timestamp()));
    }

    #[test]
    fn a_rebased_commit_counts_by_when_it_landed_not_when_it_was_authored() {
        let repo = Repo::new();
        repo.commit_dated(
            "a.rs",
            "one",
            "authored long ago, landed recently",
            "2000-01-02T03:04:05Z",
            "2020-01-02T03:04:05Z",
        );
        let adapter = GitRepository::discover(&repo.path).unwrap();
        let mut commits = Vec::new();
        let summary = adapter
            .stream_history(Some(1_500_000_000), |commit| {
                commits.push(commit);
                Ok(())
            })
            .unwrap();
        assert_eq!(summary.commits(), 1);
        assert_eq!(commits[0].timestamp(), 1_577_934_245);
    }

    #[test]
    fn a_cutoff_before_the_epoch_still_streams_every_commit() {
        let repo = Repo::new();
        repo.commit("a.rs", "one", "one");
        let adapter = GitRepository::discover(&repo.path).unwrap();
        let mut streamed = 0u32;
        let summary = adapter
            .stream_history(Some(i64::MIN), |_| {
                streamed += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!((summary.commits(), streamed), (1, 1));
    }

    #[test]
    fn history_follows_renames_and_keeps_binary_churn_unknown() {
        let repo = Repo::new();
        repo.commit("old.rs", "one\n", "initial");
        fs::rename(repo.path.join("old.rs"), repo.path.join("new.rs")).unwrap();
        git(&repo.path, ["add", "-A"]);
        git(&repo.path, ["commit", "-qm", "rename"]);
        repo.commit("image.bin", "\0\u{1}\u{2}", "binary");
        let adapter = GitRepository::discover(&repo.path).unwrap();
        let mut commits = Vec::new();
        adapter
            .stream_history(None, |commit| {
                commits.push(commit);
                Ok(())
            })
            .unwrap();
        let rename = commits
            .iter()
            .flat_map(|commit| commit.changes())
            .find(|change| change.path() == Path::new("new.rs"))
            .unwrap();
        assert_eq!(rename.previous_path(), Some(Path::new("old.rs")));
        let binary = commits
            .iter()
            .flat_map(|commit| commit.changes())
            .find(|change| change.path() == Path::new("image.bin"))
            .unwrap();
        assert_eq!(binary.added_lines(), None);
        assert_eq!(binary.deleted_lines(), None);
    }

    #[test]
    fn malformed_history_record_is_rejected() {
        let mut parser = HistoryParser::default();
        let mut accepted = |_| Ok(());
        parser
            .accept(
                b"\x1e0123456789012345678901234567890123456789",
                &mut accepted,
            )
            .unwrap();
        parser.accept(b"Name", &mut accepted).unwrap();
        parser
            .accept(b"address@example.test", &mut accepted)
            .unwrap();
        parser.accept(b"123", &mut accepted).unwrap();
        assert!(matches!(
            parser.accept(b"not-a-numstat-record", &mut accepted),
            Err(GitError::InvalidOutput(_))
        ));
    }

    #[test]
    fn malformed_history_after_valid_commit_preserves_accepted_prefix() {
        let mut parser = HistoryParser::default();
        let mut commits = Vec::new();
        let mut accepted = |commit| {
            commits.push(commit);
            Ok(())
        };
        parser
            .accept(
                b"\x1e0123456789012345678901234567890123456789",
                &mut accepted,
            )
            .unwrap();
        parser.accept(b"Name", &mut accepted).unwrap();
        parser
            .accept(b"address@example.test", &mut accepted)
            .unwrap();
        parser.accept(b"123", &mut accepted).unwrap();
        parser.accept(b"2\t1\tfirst.rs", &mut accepted).unwrap();
        parser
            .accept(
                b"\x1e1123456789012345678901234567890123456789",
                &mut accepted,
            )
            .unwrap();
        parser.accept(b"Name", &mut accepted).unwrap();
        parser
            .accept(b"address@example.test", &mut accepted)
            .unwrap();
        parser.accept(b"122", &mut accepted).unwrap();

        assert!(matches!(
            parser.accept(b"not-a-numstat-record", &mut accepted),
            Err(GitError::InvalidOutput(_))
        ));
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].changes()[0].path(), Path::new("first.rs"));
    }

    #[test]
    fn batch_reader_reuses_one_process() {
        let repo = Repo::new();
        repo.commit("a.rs", "contents", "one");
        let adapter = GitRepository::discover(&repo.path).unwrap();
        let before = adapter.git_processes();
        let object = String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD:a.rs"])
                .current_dir(&repo.path)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        let mut reader = adapter.object_reader(2).unwrap();
        assert_eq!(reader.read_object(object.trim()).unwrap(), b"contents");
        assert_eq!(reader.read_object(object.trim()).unwrap(), b"contents");
        assert_eq!(adapter.git_processes() - before, 1);
    }

    #[test]
    fn batch_reader_reads_paths_with_spaces_and_rejects_line_breaks() {
        let repo = Repo::new();
        repo.commit("with space.rs", "contents", "one");
        let adapter = GitRepository::discover(&repo.path).unwrap();
        let mut reader = adapter.object_reader(1).unwrap();
        assert_eq!(
            reader
                .read_path("HEAD", Path::new("with space.rs"))
                .unwrap(),
            b"contents"
        );
        assert!(matches!(
            reader.read_path("HEAD", Path::new("bad\nname.rs")),
            Err(GitError::UnsafeArgument(_))
        ));
    }

    #[test]
    fn batch_reader_lists_tree_files_without_another_process() {
        let repo = Repo::new();
        fs::create_dir_all(repo.path.join("nested")).unwrap();
        fs::write(repo.path.join("root.rs"), "fn root() {}\n").unwrap();
        fs::write(repo.path.join("nested/leaf.rs"), "fn leaf() {}\n").unwrap();
        git(&repo.path, ["add", "."]);
        git(&repo.path, ["commit", "-qm", "tree"]);
        let adapter = GitRepository::discover(&repo.path).unwrap();
        let mut reader = adapter.object_reader(4).unwrap();
        let processes = adapter.git_processes();
        assert_eq!(
            reader.tree_files("HEAD").unwrap(),
            [PathBuf::from("nested/leaf.rs"), PathBuf::from("root.rs")]
        );
        assert_eq!(adapter.git_processes(), processes);
    }

    #[test]
    fn rejects_shell_like_refs() {
        let repo = Repo::new();
        let adapter = GitRepository::discover(&repo.path).unwrap();
        assert!(matches!(
            adapter.merge_base("HEAD; touch pwned", "HEAD"),
            Err(GitError::UnsafeArgument(_))
        ));
        assert!(!repo.path.join("pwned").exists());
    }
}
