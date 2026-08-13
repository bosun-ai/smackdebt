//! Small, synchronous Git adapter used by the project orchestration crate.
//!
//! The adapter deliberately owns no source-analysis policy.  It only turns
//! Git's structured output into values that callers can use to build a report.
//! All commands are passed as argument vectors to `git`; refs and paths are
//! never interpolated into a shell command.

use std::collections::HashMap;
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

/// Aggregated activity for a path. `touches` counts distinct commits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileActivity {
    path: PathBuf,
    touches: u32,
}

impl FileActivity {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub const fn touches(&self) -> u32 {
        self.touches
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

    /// Stream non-merge history through one Git process and aggregate touches.
    /// The output is consumed incrementally, so history size does not require
    /// one giant Git output allocation.
    pub fn history(&self, days: u32) -> Result<Vec<FileActivity>, GitError> {
        let args = vec![
            OsString::from("log"),
            OsString::from("--no-merges"),
            OsString::from("--name-status"),
            OsString::from("--format=%H%x00"),
            OsString::from("-z"),
            OsString::from("--find-renames"),
            OsString::from(format!("--since={days} days ago")),
            OsString::from("--"),
        ];
        let mut command = self.command(args.clone());
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(GitError::Io)?;
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
        let mut history = HistoryParser::default();
        let read_result = reader.read_tokens(|token| history.accept(token));
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
            return Err(GitError::Command {
                args,
                status: status.code(),
                stderr: message,
            });
        }
        history.finish()
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
        self.processes.fetch_add(1, Ordering::Relaxed);
        Ok(output)
    }

    fn command(&self, args: Vec<OsString>) -> Command {
        let mut command = Command::new("git");
        command.args(args).current_dir(&self.root);
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
    commit_seen: bool,
    pending: Option<PendingHistory>,
    counts: HashMap<PathBuf, u32>,
}

struct PendingHistory {
    status: ChangeStatus,
    old_path: Option<PathBuf>,
}

impl HistoryParser {
    fn accept(&mut self, raw: &[u8]) -> Result<(), GitError> {
        // Git emits an extra empty record and a newline around the explicit
        // NUL format marker. Those delimiters are not paths or statuses.
        let token = raw.strip_prefix(b"\n").unwrap_or(raw);
        if token.is_empty() {
            return Ok(());
        }
        if token.len() == 40 && token.iter().all(u8::is_ascii_hexdigit) {
            if self.pending.is_some() {
                return Err(GitError::InvalidOutput(
                    "history record lacks a path".into(),
                ));
            }
            self.commit_seen = true;
            return Ok(());
        }

        if let Some(pending) = &mut self.pending {
            let path = PathBuf::from(String::from_utf8(token.to_vec())?);
            if matches!(pending.status, ChangeStatus::Renamed | ChangeStatus::Copied)
                && pending.old_path.is_none()
            {
                pending.old_path = Some(path);
                return Ok(());
            }
            if self.commit_seen {
                *self.counts.entry(path).or_default() += 1;
            }
            self.pending = None;
            return Ok(());
        }

        let tab = token.iter().position(|byte| *byte == b'\t');
        let status_token = tab.map_or(token, |index| &token[..index]);
        let code = status_token
            .first()
            .copied()
            .ok_or_else(|| GitError::InvalidOutput("empty history status".into()))?;
        let status = ChangeStatus::from_code(code, false)?;
        self.pending = Some(PendingHistory {
            status,
            old_path: None,
        });
        if let Some(index) = tab {
            let path = PathBuf::from(String::from_utf8(token[index + 1..].to_vec())?);
            if self.commit_seen {
                *self.counts.entry(path).or_default() += 1;
            }
            self.pending = None;
        }
        Ok(())
    }

    fn finish(self) -> Result<Vec<FileActivity>, GitError> {
        if self.pending.is_some() {
            return Err(GitError::InvalidOutput(
                "history record lacks a path".into(),
            ));
        }
        let mut values: Vec<_> = self
            .counts
            .into_iter()
            .map(|(path, touches)| FileActivity { path, touches })
            .collect();
        values.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(values)
    }
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
    fn history_uses_one_process_and_counts_touches() {
        let repo = Repo::new();
        repo.commit("a.rs", "one", "one");
        repo.commit("a.rs", "two", "two");
        let adapter = GitRepository::discover(&repo.path).unwrap();
        let before = adapter.git_processes();
        let activity = adapter.history(3650).unwrap();
        assert_eq!(adapter.git_processes() - before, 1);
        assert_eq!(
            activity
                .iter()
                .find(|item| item.path == Path::new("a.rs"))
                .map(|item| item.touches),
            Some(2)
        );
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
