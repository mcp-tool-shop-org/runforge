//! Start an already-installed `backprop`. Arguments are a list. Nothing goes through a shell.
//!
//! `.cmd` and `.bat` are skipped: Windows would run those through `cmd.exe`. The child is not
//! given a new environment. On Windows it is attached to a hidden pseudoconsole, so a console
//! program line-buffers its own stdout, and Stop kills the process tree with a job object.

use std::collections::VecDeque;
use std::ffi::c_void;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::process::{Child, Command, Stdio};
#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub(crate) const MISSING_TOOL: &str = "backprop is not on PATH. RunForge does not install it.";
pub(crate) const ALREADY_RUNNING: &str = "A command is already running.";
pub(crate) const NOTHING_RUNNING: &str = "No command is running.";
pub(crate) const OPEN_FOLDER: &str = "Open a folder first.";
pub(crate) const SELECT_RUN: &str = "Select a run.";
pub(crate) const NO_CHECKPOINT: &str = "The selected run has no checkpoint.";
pub(crate) const BAD_STEPS: &str = "Steps must be a positive whole number.";
pub(crate) const NEED_DATA: &str = "Choose a data file.";
pub(crate) const BAD_ARGUMENT: &str = "That value cannot be passed as an argument.";
pub(crate) const START_FAILED: &str = "backprop could not be started.";

const LOG_CAP: usize = 400;
const TOOL_FRESH: Duration = Duration::from_secs(2);
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x2000;
/// `KILL_ON_JOB_CLOSE` is rejected on the basic limit class (`ERROR_INVALID_PARAMETER`).
const JOB_EXTENDED_LIMIT: i32 = 9;

pub(crate) struct LaunchRequest {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

/// One published log piece. A revision replaces the open line. A commit closes it.
pub(crate) enum LogUpdate {
    Commit(String),
    Revise(String),
}

pub(crate) trait Session {
    fn take_lines(&mut self) -> Vec<LogUpdate>;
    fn finished(&mut self) -> Option<i32>;
    fn stop(&mut self);
}

pub(crate) fn train_args(
    model: &str,
    data: &Path,
    steps: &str,
    output: &Path,
) -> Result<Vec<String>, &'static str> {
    if data.as_os_str().is_empty() {
        return Err(NEED_DATA);
    }
    let data_text = required_text(data)?;
    let mut args = vec!["train".to_string(), "--data".to_string(), data_text];
    let model = model.trim();
    if !model.is_empty() {
        args.push("--model".to_string());
        args.push(plain_text(model)?);
    }
    let steps = steps.trim();
    if !steps.is_empty() {
        if !positive_whole(steps) {
            return Err(BAD_STEPS);
        }
        args.push("--steps".to_string());
        args.push(steps.to_string());
    }
    args.push("--output".to_string());
    args.push(output_text(output)?);
    Ok(args)
}

pub(crate) fn eval_args(run_id: &str, output: &Path) -> Result<Vec<String>, &'static str> {
    if run_id.is_empty() {
        return Err(SELECT_RUN);
    }
    let run_id = plain_text(run_id)?;
    Ok(vec![
        "eval".to_string(),
        run_id,
        "--output".to_string(),
        output_text(output)?,
    ])
}

pub(crate) fn export_args(checkpoint: &str, output: &Path) -> Result<Vec<String>, &'static str> {
    if checkpoint.is_empty() {
        return Err(NO_CHECKPOINT);
    }
    let checkpoint = plain_text(checkpoint)?;
    Ok(vec![
        "export".to_string(),
        checkpoint,
        "--output".to_string(),
        output_text(output)?,
    ])
}

fn output_text(path: &Path) -> Result<String, &'static str> {
    if path.as_os_str().is_empty() {
        return Err(OPEN_FOLDER);
    }
    required_text(path)
}

fn required_text(path: &Path) -> Result<String, &'static str> {
    let Some(text) = path.to_str() else {
        return Err(BAD_ARGUMENT);
    };
    plain_text(text)
}

fn plain_text(text: &str) -> Result<String, &'static str> {
    if text.is_empty() || text.contains('\0') || text.starts_with('-') {
        return Err(BAD_ARGUMENT);
    }
    Ok(text.to_string())
}

fn positive_whole(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && text.bytes().any(|byte| byte != b'0')
}

/// Walk `PATH`. Keep an absolute `backprop.exe`, `backprop.com`, or extensionless `backprop`.
/// Skip an empty or relative entry. A `.cmd` or `.bat` is not a result.
pub(crate) fn find_backprop(path_env: &str, pathext: &str) -> Option<PathBuf> {
    let names = tool_names(pathext);
    for dir in std::env::split_paths(path_env) {
        if dir.as_os_str().is_empty() || !dir.is_absolute() {
            continue;
        }
        for name in &names {
            let candidate = dir.join(name);
            if candidate.is_absolute() && stat_file(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn tool_names(pathext: &str) -> Vec<String> {
    let mut names = Vec::new();
    for part in pathext.split(';') {
        let ext = part.trim().to_ascii_lowercase();
        if ext == ".exe" || ext == ".com" {
            names.push(format!("backprop{ext}"));
        }
    }
    names.push("backprop".to_string());
    names
}

fn stat_file(path: &Path) -> bool {
    #[cfg(test)]
    {
        let delay = STAT_DELAY_MS.load(Ordering::Acquire);
        if delay > 0 && !STAT_SLEPT.swap(true, Ordering::AcqRel) {
            thread::sleep(Duration::from_millis(delay));
        }
        stat_threads()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(thread::current().id());
    }
    path.is_file()
}

struct ToolHit {
    generation: u64,
    at: Instant,
    path: Option<PathBuf>,
}

/// What the window may show without waiting on `PATH`.
pub(crate) enum ToolAnswer {
    /// The walk for this generation has finished.
    Ready(Option<PathBuf>),
    /// A walk is in flight and there is no finished answer yet.
    Pending,
}

struct ToolSlot {
    generation: u64,
    published: Option<ToolHit>,
    walking: Option<u64>,
}

fn tool_slot() -> &'static Mutex<ToolSlot> {
    static SLOT: Mutex<ToolSlot> = Mutex::new(ToolSlot {
        generation: 0,
        published: None,
        walking: None,
    });
    &SLOT
}

fn lock_tool() -> std::sync::MutexGuard<'static, ToolSlot> {
    tool_slot()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

/// Drop the cached hit or miss so the next lookup walks `PATH` again.
pub(crate) fn bust_tool_cache() {
    #[cfg(test)]
    let _gate = cache_gate();
    bust_tool_cache_locked();
}

fn bust_tool_cache_locked() {
    let mut guard = lock_tool();
    guard.generation = guard.generation.wrapping_add(1);
    guard.published = None;
}

/// The last finished lookup. A frame never stats `PATH` and never waits for a walk.
///
/// A result younger than two seconds is returned as it is. An older result stays
/// on screen while a background walk replaces it. After `bust_tool_cache` there
/// is no result yet, so this returns [`ToolAnswer::Pending`] until that walk
/// finishes. `is_file` runs on the worker.
pub(crate) fn tool_answer() -> ToolAnswer {
    #[cfg(test)]
    let _gate = cache_gate();
    tool_answer_locked()
}

fn tool_answer_locked() -> ToolAnswer {
    let mut guard = lock_tool();
    let generation = guard.generation;
    let published = guard
        .published
        .as_ref()
        .filter(|hit| hit.generation == generation)
        .map(|hit| (hit.at.elapsed() >= TOOL_FRESH, hit.path.clone()));
    if let Some((stale, path)) = published {
        if stale && guard.walking != Some(generation) {
            start_walk(&mut guard, generation);
        }
        return ToolAnswer::Ready(path);
    }
    if guard.walking != Some(generation) {
        start_walk(&mut guard, generation);
    }
    ToolAnswer::Pending
}

fn start_walk(guard: &mut ToolSlot, generation: u64) {
    guard.walking = Some(generation);
    thread::spawn(move || {
        #[cfg(test)]
        STAT_SLEPT.store(false, Ordering::Release);
        let path = std::env::var_os("PATH").unwrap_or_default();
        let pathext = std::env::var_os("PATHEXT").unwrap_or_default();
        let found = find_backprop(&path.to_string_lossy(), &pathext.to_string_lossy());
        finish_walk(generation, found);
    });
}

fn finish_walk(generation: u64, found: Option<PathBuf>) {
    let mut guard = lock_tool();
    if guard.generation != generation {
        if guard.walking == Some(generation) {
            guard.walking = None;
        }
        return;
    }
    guard.published = Some(ToolHit {
        generation,
        at: Instant::now(),
        path: found,
    });
    guard.walking = None;
}

#[cfg(test)]
static STAT_DELAY_MS: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
static STAT_SLEPT: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
fn cache_gate() -> std::sync::MutexGuard<'static, ()> {
    static GATE: Mutex<()> = Mutex::new(());
    GATE.lock().unwrap_or_else(|poison| poison.into_inner())
}

#[cfg(test)]
fn stat_threads() -> &'static Mutex<Vec<thread::ThreadId>> {
    static THREADS: Mutex<Vec<thread::ThreadId>> = Mutex::new(Vec::new());
    &THREADS
}

/// Make the published answer look older than the two-second window.
#[cfg(test)]
pub(crate) fn expire_tool_cache_for_test() {
    if let Some(hit) = lock_tool().published.as_mut() {
        hit.at = Instant::now() - Duration::from_secs(3);
    }
}

pub(crate) fn exit_note(code: i32) -> String {
    format!("backprop exited ({code}).")
}

pub(crate) fn remember_line(log: &mut VecDeque<String>, line: String) {
    if log.len() == LOG_CAP {
        log.pop_front();
    }
    log.push_back(line);
}

/// Fold one reader event into the visible log.
///
/// A carriage-return progress line is open until a newline commits it. An empty
/// revision is ignored so a bare `\r` cannot erase the previous line.
pub(crate) fn apply_log_update(log: &mut VecDeque<String>, open: &mut bool, update: LogUpdate) {
    match update {
        LogUpdate::Revise(text) => {
            if text.is_empty() {
                return;
            }
            if *open {
                log.pop_back();
            }
            remember_line(log, text);
            *open = true;
        }
        LogUpdate::Commit(text) => {
            if *open {
                log.pop_back();
                *open = false;
            }
            remember_line(log, text);
        }
    }
}

pub(crate) fn start_installed(request: LaunchRequest) -> Result<Box<dyn Session>, &'static str> {
    Ok(Box::new(spawn_session(request)?))
}

pub(crate) fn spawn_session(request: LaunchRequest) -> Result<LiveSession, &'static str> {
    #[cfg(windows)]
    {
        spawn_console_session(request)
    }
    #[cfg(not(windows))]
    {
        spawn_piped_session(request)
    }
}

#[cfg(not(windows))]
fn spawn_piped_session(request: LaunchRequest) -> Result<LiveSession, &'static str> {
    let mut command = Command::new(&request.program);
    command
        .args(&request.args)
        .current_dir(&request.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|_| START_FAILED)?;
    let lines = Arc::new(Mutex::new(VecDeque::new()));
    let readers_left = Arc::new(AtomicUsize::new(0));
    let mut readers = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        readers.push(spawn_reader(stdout, &lines, &readers_left));
    }
    if let Some(stderr) = child.stderr.take() {
        readers.push(spawn_reader(stderr, &lines, &readers_left));
    }
    Ok(LiveSession {
        child: Some(child),
        lines,
        readers,
        readers_left,
        exit_code: None,
    })
}

const PARTIAL_CAP: usize = 4096;

/// Read state for one pipe.
///
/// Backprop's progress bar is a flushed `\r` prefix, not a newline. The current
/// bar is published as soon as the buffered bytes run out, and the pending bytes
/// stay so the next chunk extends that same line. A `\r` at the end of a buffer
/// is consumed (`held_cr`) so the next read can tell CRLF from a new revision
/// without spinning on the same byte. A 4096-byte piece is committed so a later
/// revision cannot delete it.
struct LogPump {
    pending: Vec<u8>,
    progress: bool,
    held_cr: bool,
}

impl LogPump {
    fn new() -> Self {
        Self {
            pending: Vec::new(),
            progress: false,
            held_cr: false,
        }
    }

    /// One update, or `None` at EOF or on an IO error. A non-UTF-8 byte stays in the line.
    /// An IO error drops the unfinished piece, matching the old reader.
    fn next(&mut self, reader: &mut impl BufRead) -> Option<LogUpdate> {
        loop {
            let (consumed, update, available) = match reader.fill_buf() {
                Err(_) => return None,
                Ok([]) => return self.finish(),
                Ok(buf) => {
                    let available = buf.len();
                    let (consumed, update) = self.step(buf);
                    (consumed, update, available)
                }
            };
            reader.consume(consumed);
            if let Some(update) = update {
                return Some(update);
            }
            if consumed == 0 {
                return None;
            }
            if self.progress && !self.pending.is_empty() && consumed == available {
                return Some(LogUpdate::Revise(visible_line(&self.pending)));
            }
        }
    }

    fn finish(&mut self) -> Option<LogUpdate> {
        self.held_cr = false;
        if self.pending.is_empty() {
            self.progress = false;
            None
        } else {
            Some(self.take_commit())
        }
    }

    fn take_commit(&mut self) -> LogUpdate {
        let text = visible_line(&self.pending);
        self.pending.clear();
        self.progress = false;
        LogUpdate::Commit(text)
    }

    /// Publish the text before a bare carriage return, then start a revision.
    fn break_for_progress(&mut self) -> Option<LogUpdate> {
        let update = if self.pending.is_empty() {
            None
        } else {
            let text = visible_line(&self.pending);
            self.pending.clear();
            Some(if self.progress {
                LogUpdate::Revise(text)
            } else {
                LogUpdate::Commit(text)
            })
        };
        self.progress = true;
        update
    }

    fn step(&mut self, buf: &[u8]) -> (usize, Option<LogUpdate>) {
        let mut index = 0;
        if self.held_cr {
            self.held_cr = false;
            if buf[0] == b'\n' {
                return (1, Some(self.take_commit()));
            }
            if let Some(update) = self.break_for_progress() {
                return (0, Some(update));
            }
        }
        while index < buf.len() {
            let byte = buf[index];
            if byte == b'\r' {
                let followed = index + 1 < buf.len();
                if followed && buf[index + 1] == b'\n' {
                    return (index + 2, Some(self.take_commit()));
                }
                if followed {
                    index += 1;
                    if let Some(update) = self.break_for_progress() {
                        return (index, Some(update));
                    }
                    continue;
                }
                self.held_cr = true;
                return (index + 1, None);
            }
            if byte == b'\n' {
                return (index + 1, Some(self.take_commit()));
            }
            self.pending.push(byte);
            index += 1;
            if self.pending.len() == PARTIAL_CAP {
                return (index, Some(self.take_commit()));
            }
        }
        (buf.len(), None)
    }
}

fn spawn_reader<R>(
    reader: R,
    lines: &Arc<Mutex<VecDeque<LogUpdate>>>,
    readers_left: &Arc<AtomicUsize>,
) -> thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
    readers_left.fetch_add(1, Ordering::Relaxed);
    let lines = Arc::clone(lines);
    let readers_left = Arc::clone(readers_left);
    thread::spawn(move || {
        let mut buffered = BufReader::new(reader);
        let mut pump = LogPump::new();
        while let Some(update) = pump.next(&mut buffered) {
            push_shared(&lines, update);
        }
        readers_left.fetch_sub(1, Ordering::Relaxed);
    })
}

/// Drop console control sequences so a pseudoconsole stream stays readable.
fn visible_line(bytes: &[u8]) -> String {
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != 0x1b {
            out.push(bytes[index]);
            index += 1;
            continue;
        }
        index += 1;
        if index >= bytes.len() {
            break;
        }
        match bytes[index] {
            b'[' => {
                index += 1;
                while index < bytes.len() && !(0x40..=0x7e).contains(&bytes[index]) {
                    index += 1;
                }
                if index < bytes.len() {
                    index += 1;
                }
            }
            b']' | b'P' | b'X' | b'^' | b'_' => {
                index += 1;
                while index < bytes.len() {
                    if bytes[index] == 0x07 {
                        index += 1;
                        break;
                    }
                    if bytes[index] == 0x1b && index + 1 < bytes.len() && bytes[index + 1] == b'\\'
                    {
                        index += 2;
                        break;
                    }
                    index += 1;
                }
            }
            _ => index += 1,
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn push_shared(lines: &Mutex<VecDeque<LogUpdate>>, update: LogUpdate) {
    if let LogUpdate::Revise(text) = &update
        && text.is_empty()
    {
        return;
    }
    let mut guard = lines.lock().unwrap_or_else(|poison| poison.into_inner());
    if matches!(update, LogUpdate::Revise(_)) && matches!(guard.back(), Some(LogUpdate::Revise(_)))
    {
        guard.pop_back();
    }
    if guard.len() == LOG_CAP {
        guard.pop_front();
    }
    guard.push_back(update);
}

pub(crate) struct LiveSession {
    #[cfg(not(windows))]
    child: Option<Child>,
    #[cfg(windows)]
    process: Option<TrackedProcess>,
    #[cfg(windows)]
    console: Option<HiddenConsole>,
    lines: Arc<Mutex<VecDeque<LogUpdate>>>,
    readers: Vec<thread::JoinHandle<()>>,
    readers_left: Arc<AtomicUsize>,
    exit_code: Option<i32>,
    #[cfg(windows)]
    job: Option<Job>,
}

impl Session for LiveSession {
    fn take_lines(&mut self) -> Vec<LogUpdate> {
        let mut guard = self
            .lines
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        guard.drain(..).collect()
    }

    fn finished(&mut self) -> Option<i32> {
        if let Some(code) = self.exit_code {
            return Some(code);
        }
        #[cfg(windows)]
        {
            let handle = self.process.as_ref()?.handle;
            let code = poll_process(handle)?;
            // Closing the pseudoconsole lets the output pipe end, so the reader
            // can finish the last line. The process has already exited.
            self.console.take();
            self.wait_for_readers();
            self.exit_code = Some(code);
            Some(code)
        }
        #[cfg(not(windows))]
        {
            let child = self.child.as_mut()?;
            let status = match child.try_wait() {
                Ok(Some(status)) => status,
                Ok(None) => return None,
                Err(_) => {
                    self.exit_code = Some(-1);
                    return Some(-1);
                }
            };
            self.wait_for_readers();
            let code = status.code().unwrap_or(-1);
            self.exit_code = Some(code);
            Some(code)
        }
    }

    fn stop(&mut self) {
        #[cfg(windows)]
        {
            if let Some(mut job) = self.job.take() {
                job.terminate();
            }
            if let Some(process) = self.process.as_ref() {
                unsafe {
                    TerminateProcess(process.handle, 1);
                }
            }
        }
        #[cfg(not(windows))]
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
        }
    }
}

impl Drop for LiveSession {
    fn drop(&mut self) {
        self.stop();
        #[cfg(windows)]
        if let Some(process) = self.process.as_ref() {
            wait_process(process.handle);
        }
        #[cfg(not(windows))]
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
        }
        #[cfg(windows)]
        {
            self.console.take();
        }
        // Detach. Joining would stall the window if a pipe stayed open.
        self.readers.clear();
    }
}

impl LiveSession {
    fn wait_for_readers(&self) {
        let start = Instant::now();
        while self.readers_left.load(Ordering::Relaxed) > 0
            && start.elapsed() < Duration::from_millis(500)
        {
            thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(windows)]
const EXTENDED_STARTUPINFO_PRESENT: u32 = 0x0008_0000;
/// The process is assigned to the job before it runs. Assigning a running
/// pseudoconsole process makes it fail at startup.
const CREATE_SUSPENDED: u32 = 0x0000_0004;
#[cfg(windows)]
const STARTF_USESTDHANDLES: u32 = 0x0000_0100;
#[cfg(windows)]
const PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE: usize = 0x0002_0016;
#[cfg(windows)]
const WAIT_OBJECT_0: u32 = 0;
#[cfg(windows)]
const WAIT_TIMEOUT: u32 = 258;

/// Win32 `COORD`. Spawn passes the packed size. The layout test locks this shape.
#[cfg(windows)]
#[cfg_attr(not(test), allow(dead_code))]
#[repr(C)]
#[derive(Clone, Copy)]
struct Coord {
    x: i16,
    y: i16,
}

#[cfg(windows)]
#[repr(C)]
struct SecurityAttributes {
    length: u32,
    _pad: u32,
    descriptor: *mut c_void,
    inherit: i32,
    _pad2: u32,
}

#[cfg(windows)]
#[repr(C)]
struct StartupInfoW {
    cb: u32,
    _pad0: u32,
    reserved: *mut u16,
    desktop: *mut u16,
    title: *mut u16,
    x: u32,
    y: u32,
    x_size: u32,
    y_size: u32,
    x_count: u32,
    y_count: u32,
    fill: u32,
    flags: u32,
    show: u16,
    reserved2_len: u16,
    _pad1: u32,
    reserved2: *mut u8,
    std_input: *mut c_void,
    std_output: *mut c_void,
    std_error: *mut c_void,
}

#[cfg(windows)]
#[repr(C)]
struct StartupInfoExW {
    startup: StartupInfoW,
    attributes: *mut c_void,
}

#[cfg(windows)]
#[repr(C)]
struct ProcessInformation {
    process: *mut c_void,
    thread: *mut c_void,
    pid: u32,
    tid: u32,
}

#[cfg(windows)]
struct TrackedProcess {
    handle: *mut c_void,
}

#[cfg(windows)]
impl Drop for TrackedProcess {
    fn drop(&mut self) {
        close_handle(self.handle);
    }
}

#[cfg(windows)]
struct HiddenConsole {
    handle: *mut c_void,
    input_write: *mut c_void,
}

#[cfg(windows)]
impl Drop for HiddenConsole {
    fn drop(&mut self) {
        unsafe {
            close_handle(self.input_write);
            if !self.handle.is_null() {
                ClosePseudoConsole(self.handle);
            }
        }
    }
}

#[cfg(windows)]
fn close_handle(handle: *mut c_void) {
    if !handle.is_null() {
        unsafe {
            CloseHandle(handle);
        }
    }
}

#[cfg(windows)]
fn poll_process(handle: *mut c_void) -> Option<i32> {
    unsafe {
        match WaitForSingleObject(handle, 0) {
            WAIT_TIMEOUT => None,
            WAIT_OBJECT_0 => Some(process_exit_code(handle)),
            _ => Some(-1),
        }
    }
}

#[cfg(windows)]
fn wait_process(handle: *mut c_void) {
    unsafe {
        WaitForSingleObject(handle, 0xFFFF_FFFF);
    }
}

#[cfg(windows)]
fn process_exit_code(handle: *mut c_void) -> i32 {
    let mut code = 0u32;
    unsafe {
        if GetExitCodeProcess(handle, &mut code) == 0 {
            return -1;
        }
    }
    code as i32
}

#[cfg(windows)]
fn wide_units(text: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    text.encode_wide().chain(std::iter::once(0)).collect()
}

/// CreateProcess keeps a trailing slash only on a drive root.
#[cfg(windows)]
fn cwd_units(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    let mut units: Vec<u16> = path.as_os_str().encode_wide().collect();
    let slash = |unit: u16| unit == u16::from(b'\\') || unit == u16::from(b'/');
    while units.len() > 3 && units.last().copied().is_some_and(slash) {
        units.pop();
    }
    units.push(0);
    units
}

#[cfg(windows)]
fn push_quoted(out: &mut Vec<u16>, text: &std::ffi::OsStr) {
    use std::os::windows::ffi::OsStrExt;
    let units: Vec<u16> = text.encode_wide().collect();
    let quote = units.is_empty()
        || units.iter().any(|unit| {
            *unit == u16::from(b' ') || *unit == u16::from(b'\t') || *unit == u16::from(b'"')
        });
    if !quote {
        out.extend(units);
        return;
    }
    out.push(u16::from(b'"'));
    let mut slashes = 0usize;
    for unit in units {
        if unit == u16::from(b'\\') {
            slashes += 1;
            continue;
        }
        if unit == u16::from(b'"') {
            for _ in 0..(slashes * 2 + 1) {
                out.push(u16::from(b'\\'));
            }
            out.push(unit);
            slashes = 0;
            continue;
        }
        for _ in 0..slashes {
            out.push(u16::from(b'\\'));
        }
        slashes = 0;
        out.push(unit);
    }
    for _ in 0..(slashes * 2) {
        out.push(u16::from(b'\\'));
    }
    out.push(u16::from(b'"'));
}

#[cfg(windows)]
fn command_line_units(program: &Path, args: &[String]) -> Vec<u16> {
    let mut units = Vec::new();
    push_quoted(&mut units, program.as_os_str());
    for arg in args {
        units.push(u16::from(b' '));
        push_quoted(&mut units, std::ffi::OsStr::new(arg));
    }
    units.push(0);
    units
}

#[cfg(all(windows, test))]
fn command_line_string(program: &Path, args: &[String]) -> String {
    String::from_utf16_lossy(&command_line_units(program, args))
        .trim_end_matches('\0')
        .to_string()
}

/// Attach the child to a hidden pseudoconsole. No shell and no new environment.
///
/// A pipe is not a console, so CPython would block-buffer stdout until the first
/// flush. The pseudoconsole is the console the child sees. This process reads
/// the console output pipe. `CREATE_NO_WINDOW` is not used: it would detach that
/// console.
#[cfg(windows)]
fn spawn_console_session(request: LaunchRequest) -> Result<LiveSession, &'static str> {
    use std::os::windows::io::{FromRawHandle, OwnedHandle};

    /// Closes every handle still owned here. Success takes the live handles out first.
    struct OwnedSpawn {
        input_read: *mut c_void,
        input_write: *mut c_void,
        output_read: *mut c_void,
        output_write: *mut c_void,
        console: *mut c_void,
        list: *mut u8,
        layout: Option<std::alloc::Layout>,
        list_ready: bool,
        process: *mut c_void,
        thread: *mut c_void,
    }

    impl Drop for OwnedSpawn {
        fn drop(&mut self) {
            close_handle(self.input_read);
            close_handle(self.input_write);
            close_handle(self.output_read);
            close_handle(self.output_write);
            close_handle(self.thread);
            if !self.process.is_null() {
                unsafe {
                    TerminateProcess(self.process, 1);
                }
                close_handle(self.process);
            }
            if self.list_ready && !self.list.is_null() {
                unsafe {
                    DeleteProcThreadAttributeList(self.list.cast());
                }
            }
            if let Some(layout) = self.layout
                && !self.list.is_null()
            {
                unsafe {
                    std::alloc::dealloc(self.list, layout);
                }
            }
            if !self.console.is_null() {
                unsafe {
                    ClosePseudoConsole(self.console);
                }
            }
        }
    }

    let mut owned = OwnedSpawn {
        input_read: std::ptr::null_mut(),
        input_write: std::ptr::null_mut(),
        output_read: std::ptr::null_mut(),
        output_write: std::ptr::null_mut(),
        console: std::ptr::null_mut(),
        list: std::ptr::null_mut(),
        layout: None,
        list_ready: false,
        process: std::ptr::null_mut(),
        thread: std::ptr::null_mut(),
    };
    unsafe {
        let mut security = SecurityAttributes {
            length: u32::try_from(size_of::<SecurityAttributes>()).unwrap_or(0),
            _pad: 0,
            descriptor: std::ptr::null_mut(),
            inherit: 1,
            _pad2: 0,
        };
        let pipe_in = CreatePipe(
            &raw mut owned.input_read,
            &raw mut owned.input_write,
            &raw mut security,
            0,
        );
        let pipe_out = CreatePipe(
            &raw mut owned.output_read,
            &raw mut owned.output_write,
            &raw mut security,
            0,
        );
        let created = CreatePseudoConsole(
            (50u32 << 16) | 240,
            owned.input_read,
            owned.output_write,
            0,
            &raw mut owned.console,
        );
        // Conhost already holds its own copies. These ends belong to this process only.
        close_handle(owned.input_read);
        owned.input_read = std::ptr::null_mut();
        close_handle(owned.output_write);
        owned.output_write = std::ptr::null_mut();
        if pipe_in == 0 || pipe_out == 0 || created < 0 || owned.console.is_null() {
            return Err(START_FAILED);
        }
        let mut bytes = 0usize;
        InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &raw mut bytes);
        let Ok(layout) = std::alloc::Layout::from_size_align(bytes, 16) else {
            return Err(START_FAILED);
        };
        owned.layout = Some(layout);
        owned.list = std::alloc::alloc(layout);
        if owned.list.is_null() {
            return Err(START_FAILED);
        }
        let init = InitializeProcThreadAttributeList(owned.list.cast(), 1, 0, &raw mut bytes);
        if init == 0 {
            return Err(START_FAILED);
        }
        owned.list_ready = true;
        // The handle value itself. A pointer to the local is a different address, and
        // the child then starts without this console.
        let updated = UpdateProcThreadAttribute(
            owned.list.cast(),
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
            owned.console,
            size_of::<*mut c_void>(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if updated == 0 {
            return Err(START_FAILED);
        }
        let mut startup: StartupInfoExW = std::mem::zeroed();
        startup.startup.cb = u32::try_from(size_of::<StartupInfoExW>()).unwrap_or(0);
        startup.attributes = owned.list.cast();
        // Invalid standard handles force the child onto the pseudoconsole. Leaving
        // them unset keeps this process's console, so the pipe never sees the banner.
        startup.startup.flags = STARTF_USESTDHANDLES;
        let invalid = (-1isize) as *mut c_void;
        startup.startup.std_input = invalid;
        startup.startup.std_output = invalid;
        startup.startup.std_error = invalid;
        let program = wide_units(request.program.as_os_str());
        let mut command = command_line_units(&request.program, &request.args);
        let cwd = cwd_units(&request.cwd);
        let mut info = ProcessInformation {
            process: std::ptr::null_mut(),
            thread: std::ptr::null_mut(),
            pid: 0,
            tid: 0,
        };
        let started = CreateProcessW(
            program.as_ptr(),
            command.as_mut_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED,
            std::ptr::null_mut(),
            cwd.as_ptr(),
            (&raw mut startup).cast(),
            &raw mut info,
        );
        if started == 0 {
            return Err(START_FAILED);
        }
        owned.process = info.process;
        owned.thread = info.thread;
        let job = assign_kill_on_close(owned.process);
        let resumed = ResumeThread(owned.thread);
        if resumed == u32::MAX {
            return Err(START_FAILED);
        }
        let output_read = owned.output_read;
        owned.output_read = std::ptr::null_mut();
        let input_write = owned.input_write;
        owned.input_write = std::ptr::null_mut();
        let console = owned.console;
        owned.console = std::ptr::null_mut();
        let process = owned.process;
        owned.process = std::ptr::null_mut();
        let thread = owned.thread;
        owned.thread = std::ptr::null_mut();
        close_handle(thread);
        let output = std::fs::File::from(OwnedHandle::from_raw_handle(output_read));
        let lines = Arc::new(Mutex::new(VecDeque::new()));
        let readers_left = Arc::new(AtomicUsize::new(0));
        let readers = vec![spawn_reader(output, &lines, &readers_left)];
        Ok(LiveSession {
            process: Some(TrackedProcess { handle: process }),
            console: Some(HiddenConsole {
                handle: console,
                input_write,
            }),
            lines,
            readers,
            readers_left,
            exit_code: None,
            job,
        })
    }
}
#[cfg(windows)]
struct Job {
    handle: *mut c_void,
}

#[cfg(windows)]
impl Job {
    fn terminate(&mut self) {
        // SAFETY: `handle` is an open job object created in this process.
        unsafe {
            TerminateJobObject(self.handle, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: `handle` is owned here and is closed once.
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

#[cfg(windows)]
fn assign_kill_on_close(process: *mut c_void) -> Option<Job> {
    // SAFETY: the job handle is checked for null, the limit struct matches the Win32 layout,
    // and the process handle comes from the child we just spawned. On failure the job is closed.
    unsafe {
        let handle = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
        if handle.is_null() {
            return None;
        }
        let mut info = JobExtendedLimit {
            basic: JobBasicLimit {
                per_process_user_time_limit: 0,
                per_job_user_time_limit: 0,
                limit_flags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                _pad0: 0,
                minimum_working_set_size: 0,
                maximum_working_set_size: 0,
                active_process_limit: 0,
                _pad1: 0,
                affinity: 0,
                priority_class: 0,
                scheduling_class: 0,
            },
            read_operation_count: 0,
            write_operation_count: 0,
            other_operation_count: 0,
            read_transfer_count: 0,
            write_transfer_count: 0,
            other_transfer_count: 0,
            process_memory_limit: 0,
            job_memory_limit: 0,
            peak_process_memory_used: 0,
            peak_job_memory_used: 0,
        };
        let set = SetInformationJobObject(
            handle,
            JOB_EXTENDED_LIMIT,
            (&raw mut info).cast(),
            u32::try_from(size_of::<JobExtendedLimit>()).unwrap_or(0),
        );
        if set == 0 || AssignProcessToJobObject(handle, process) == 0 {
            CloseHandle(handle);
            return None;
        }
        Some(Job { handle })
    }
}

#[cfg(windows)]
#[repr(C)]
struct JobBasicLimit {
    per_process_user_time_limit: i64,
    per_job_user_time_limit: i64,
    limit_flags: u32,
    _pad0: u32,
    minimum_working_set_size: usize,
    maximum_working_set_size: usize,
    active_process_limit: u32,
    _pad1: u32,
    affinity: usize,
    priority_class: u32,
    scheduling_class: u32,
}

/// Win32 `JOBOBJECT_EXTENDED_LIMIT_INFORMATION`. The kill-on-close flag lives here.
#[cfg(windows)]
#[repr(C)]
struct JobExtendedLimit {
    basic: JobBasicLimit,
    read_operation_count: u64,
    write_operation_count: u64,
    other_operation_count: u64,
    read_transfer_count: u64,
    write_transfer_count: u64,
    other_transfer_count: u64,
    process_memory_limit: usize,
    job_memory_limit: usize,
    peak_process_memory_used: usize,
    peak_job_memory_used: usize,
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateJobObjectW(attributes: *mut c_void, name: *const u16) -> *mut c_void;
    fn SetInformationJobObject(job: *mut c_void, class: i32, info: *mut c_void, length: u32)
    -> i32;
    fn AssignProcessToJobObject(job: *mut c_void, process: *mut c_void) -> i32;
    fn TerminateJobObject(job: *mut c_void, code: u32) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
    fn CreatePipe(
        read: *mut *mut c_void,
        write: *mut *mut c_void,
        attributes: *mut SecurityAttributes,
        size: u32,
    ) -> i32;
    fn CreatePseudoConsole(
        size: u32,
        input: *mut c_void,
        output: *mut c_void,
        flags: u32,
        console: *mut *mut c_void,
    ) -> i32;
    fn ClosePseudoConsole(console: *mut c_void);
    fn InitializeProcThreadAttributeList(
        list: *mut c_void,
        count: u32,
        flags: u32,
        bytes: *mut usize,
    ) -> i32;
    fn UpdateProcThreadAttribute(
        list: *mut c_void,
        flags: u32,
        attribute: usize,
        value: *mut c_void,
        size: usize,
        previous: *mut c_void,
        returned: *mut usize,
    ) -> i32;
    fn DeleteProcThreadAttributeList(list: *mut c_void);
    fn CreateProcessW(
        application: *const u16,
        command: *mut u16,
        process_attributes: *mut c_void,
        thread_attributes: *mut c_void,
        inherit: i32,
        flags: u32,
        environment: *mut c_void,
        cwd: *const u16,
        startup: *mut StartupInfoW,
        process: *mut ProcessInformation,
    ) -> i32;
    fn WaitForSingleObject(handle: *mut c_void, millis: u32) -> u32;
    fn GetExitCodeProcess(handle: *mut c_void, code: *mut u32) -> i32;
    fn TerminateProcess(handle: *mut c_void, code: u32) -> i32;
    fn ResumeThread(thread: *mut c_void) -> u32;
}

#[cfg(test)]
mod tests {
    use super::{
        BAD_ARGUMENT, BAD_STEPS, LOG_CAP, LaunchRequest, LogPump, LogUpdate, NEED_DATA,
        NO_CHECKPOINT, OPEN_FOLDER, PARTIAL_CAP, SELECT_RUN, START_FAILED, Session, ToolAnswer,
        apply_log_update, bust_tool_cache, eval_args, exit_note, export_args, find_backprop,
        push_shared, remember_line, spawn_session, start_installed, tool_answer, train_args,
    };
    use std::collections::VecDeque;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Mutex};

    fn scratch(name: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "runforge-launch-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn file_name(path: Option<&Path>) -> Option<&str> {
        path.and_then(Path::file_name)
            .and_then(|name| name.to_str())
    }

    fn path_env(dirs: &[&Path]) -> String {
        let Ok(joined) = std::env::join_paths(dirs) else {
            panic!("path");
        };
        joined.to_string_lossy().into_owned()
    }

    #[test]
    fn train_eval_and_export_build_exact_argv() {
        let train = train_args("small", Path::new("notes.json"), "12", Path::new("out")).unwrap();
        assert_eq!(
            train,
            vec![
                "train",
                "--data",
                "notes.json",
                "--model",
                "small",
                "--steps",
                "12",
                "--output",
                "out"
            ]
        );
        let plain = train_args("  ", Path::new("notes.json"), " ", Path::new("out")).unwrap();
        assert_eq!(
            plain,
            vec!["train", "--data", "notes.json", "--output", "out"]
        );
        let trimmed = train_args(
            "  small  ",
            Path::new("notes.json"),
            " 01 ",
            Path::new("out"),
        )
        .unwrap();
        assert_eq!(
            trimmed,
            vec![
                "train",
                "--data",
                "notes.json",
                "--model",
                "small",
                "--steps",
                "01",
                "--output",
                "out"
            ]
        );
        assert_eq!(
            eval_args("newer", Path::new("out")).unwrap(),
            vec!["eval", "newer", "--output", "out"]
        );
        assert_eq!(
            export_args("ckpt", Path::new("out")).unwrap(),
            vec!["export", "ckpt", "--output", "out"]
        );
        assert_eq!(exit_note(0), "backprop exited (0).");
    }

    #[test]
    fn bad_fields_are_sentences() {
        assert_eq!(
            train_args("", Path::new(""), "", Path::new("out")),
            Err(NEED_DATA)
        );
        assert_eq!(
            train_args("", Path::new("notes.json"), "0", Path::new("out")),
            Err(BAD_STEPS)
        );
        assert_eq!(
            train_args("", Path::new("notes.json"), "00", Path::new("out")),
            Err(BAD_STEPS)
        );
        assert_eq!(
            train_args("", Path::new("notes.json"), "-1", Path::new("out")),
            Err(BAD_STEPS)
        );
        assert_eq!(
            train_args("", Path::new("notes.json"), "+1", Path::new("out")),
            Err(BAD_STEPS)
        );
        assert_eq!(
            train_args("", Path::new("notes.json"), "1.5", Path::new("out")),
            Err(BAD_STEPS)
        );
        assert_eq!(
            train_args("", Path::new("notes.json"), "1e2", Path::new("out")),
            Err(BAD_STEPS)
        );
        assert_eq!(
            train_args("-model", Path::new("notes.json"), "", Path::new("out")),
            Err(BAD_ARGUMENT)
        );
        assert_eq!(
            train_args("ok", Path::new("-notes"), "", Path::new("out")),
            Err(BAD_ARGUMENT)
        );
        assert_eq!(
            train_args("ok", Path::new("a\0b"), "", Path::new("out")),
            Err(BAD_ARGUMENT)
        );
        assert_eq!(
            train_args("ok", Path::new("notes.json"), "", Path::new("")),
            Err(OPEN_FOLDER)
        );
        assert_eq!(
            train_args("ok", Path::new("notes.json"), "", Path::new("-out")),
            Err(BAD_ARGUMENT)
        );
        assert_eq!(eval_args("", Path::new("out")), Err(SELECT_RUN));
        assert_eq!(eval_args("-run", Path::new("out")), Err(BAD_ARGUMENT));
        assert_eq!(eval_args("run\0id", Path::new("out")), Err(BAD_ARGUMENT));
        assert_eq!(export_args("", Path::new("out")), Err(NO_CHECKPOINT));
        assert_eq!(export_args("-ckpt", Path::new("out")), Err(BAD_ARGUMENT));
        assert_eq!(export_args("ck\0pt", Path::new("out")), Err(BAD_ARGUMENT));
    }

    #[test]
    fn the_log_keeps_the_newest_lines() {
        let mut log = VecDeque::new();
        for index in 0..=LOG_CAP {
            remember_line(&mut log, index.to_string());
        }
        assert_eq!(log.len(), LOG_CAP);
        assert_eq!(log.front().map(String::as_str), Some("1"));
        let newest = LOG_CAP.to_string();
        assert_eq!(log.back().map(String::as_str), Some(newest.as_str()));
    }

    #[test]
    fn a_poisoned_log_still_keeps_the_line() {
        use std::panic::AssertUnwindSafe;
        let lines = Arc::new(Mutex::new(VecDeque::new()));
        let clone = Arc::clone(&lines);
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let _guard = clone.lock().unwrap_or_else(|poison| poison.into_inner());
            panic!("poison");
        }));
        push_shared(&lines, LogUpdate::Commit("kept".to_string()));
        let guard = lines.lock().unwrap_or_else(|poison| poison.into_inner());
        let kept = matches!(guard.back(), Some(LogUpdate::Commit(text)) if text == "kept");
        assert!(kept);
    }

    #[test]
    fn path_lookup_prefers_the_exe_and_skips_scripts() {
        let exe_dir = scratch("exe");
        let cmd_dir = scratch("cmd");
        let bare_dir = scratch("bare");
        let com_dir = scratch("com");
        let nested = scratch("nested");
        std::fs::write(exe_dir.join("backprop.exe"), b"").unwrap();
        std::fs::write(exe_dir.join("backprop"), b"").unwrap();
        std::fs::write(cmd_dir.join("backprop.cmd"), b"").unwrap();
        std::fs::write(cmd_dir.join("backprop.bat"), b"").unwrap();
        std::fs::create_dir(cmd_dir.join("backprop.exe")).unwrap();
        std::fs::write(bare_dir.join("backprop"), b"").unwrap();
        std::fs::write(com_dir.join("backprop.com"), b"").unwrap();
        std::fs::write(com_dir.join("backprop.exe"), b"").unwrap();
        let sub = nested.join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("backprop.exe"), b"").unwrap();

        let found = find_backprop(&path_env(&[exe_dir.as_path()]), ".COM;.EXE;.BAT;.CMD");
        assert_eq!(file_name(found.as_deref()), Some("backprop.exe"));
        assert!(find_backprop(&path_env(&[cmd_dir.as_path()]), ".CMD;.BAT;.EXE").is_none());
        let bare = find_backprop(&path_env(&[bare_dir.as_path()]), ".EXE");
        assert_eq!(file_name(bare.as_deref()), Some("backprop"));
        let com = find_backprop(&path_env(&[com_dir.as_path()]), ".COM;.EXE");
        assert_eq!(file_name(com.as_deref()), Some("backprop.com"));
        let later = find_backprop(
            &path_env(&[cmd_dir.as_path(), exe_dir.as_path()]),
            ".CMD;.EXE",
        );
        assert_eq!(file_name(later.as_deref()), Some("backprop.exe"));
        assert!(find_backprop(&path_env(&[nested.as_path()]), ".EXE").is_none());
        assert!(find_backprop("", ".EXE").is_none());
    }

    struct RemoveOnDrop<'a>(&'a Path);

    impl Drop for RemoveOnDrop<'_> {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0);
        }
    }

    #[test]
    fn blank_and_relative_entries_are_not_a_tool() {
        assert!(find_backprop(";", ".EXE").is_none());

        let relative = format!("runforge-rel-{}", std::process::id());
        let relative_dir = PathBuf::from(&relative);
        let _cleanup = RemoveOnDrop(&relative_dir);
        let _ = std::fs::remove_dir_all(&relative_dir);
        std::fs::create_dir(&relative_dir).unwrap();
        std::fs::write(relative_dir.join("backprop.exe"), b"").unwrap();
        assert!(find_backprop(&relative, ".EXE").is_none());

        let exe_dir = scratch("abs-exe");
        std::fs::write(exe_dir.join("backprop.exe"), b"").unwrap();
        let listed = format!(";{relative};{}", path_env(&[exe_dir.as_path()]));
        let found = find_backprop(&listed, ".EXE");
        let absolute = found.as_ref().is_some_and(|path| path.is_absolute());
        assert!(absolute);
        assert_eq!(file_name(found.as_deref()), Some("backprop.exe"));
    }

    fn lines_from(bytes: &[u8]) -> Vec<String> {
        let mut cursor = std::io::Cursor::new(bytes);
        let mut pump = LogPump::new();
        let mut log = VecDeque::new();
        let mut open = false;
        while let Some(update) = pump.next(&mut cursor) {
            apply_log_update(&mut log, &mut open, update);
        }
        log.into_iter().collect()
    }

    struct PieceReader {
        parts: Vec<&'static [u8]>,
        index: usize,
        off: usize,
    }

    impl std::io::Read for PieceReader {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            if self.index >= self.parts.len() {
                return Ok(0);
            }
            let part = self.parts[self.index];
            let rest = part.len().saturating_sub(self.off);
            if rest == 0 {
                self.index += 1;
                self.off = 0;
                return self.read(out);
            }
            let count = rest.min(out.len());
            out[..count].copy_from_slice(&part[self.off..self.off + count]);
            self.off += count;
            if self.off == part.len() {
                self.index += 1;
                self.off = 0;
            }
            Ok(count)
        }
    }

    fn lines_from_pieces(parts: Vec<&'static [u8]>) -> (Vec<String>, Vec<LogUpdate>) {
        let mut reader = std::io::BufReader::new(PieceReader {
            parts,
            index: 0,
            off: 0,
        });
        let mut pump = LogPump::new();
        let mut log = VecDeque::new();
        let mut open = false;
        let mut seen = Vec::new();
        while let Some(update) = pump.next(&mut reader) {
            seen.push(match &update {
                LogUpdate::Commit(text) => LogUpdate::Commit(text.clone()),
                LogUpdate::Revise(text) => LogUpdate::Revise(text.clone()),
            });
            apply_log_update(&mut log, &mut open, update);
        }
        (log.into_iter().collect(), seen)
    }

    #[test]
    fn a_bad_byte_does_not_drop_the_rest_of_the_log() {
        let lines = lines_from(b"ok\n\xff\nnext\n");
        let later = lines.iter().skip(1).any(|line| line == "next");
        assert!(later);
    }

    #[test]
    fn a_carriage_return_bar_replaces_itself_and_shows_before_newline() {
        let lines = lines_from(b"banner\n\rbar-one\rbar-two\n");
        let finished = lines.len() == 2 && lines[0] == "banner" && lines[1] == "bar-two";
        assert!(finished);
        let mut cursor = std::io::Cursor::new(&b"banner\n\rbar-one"[..]);
        let mut pump = LogPump::new();
        let mut saw_bar = false;
        while let Some(update) = pump.next(&mut cursor) {
            if let LogUpdate::Revise(text) = &update
                && text == "bar-one"
            {
                saw_bar = true;
                break;
            }
        }
        assert!(saw_bar);
    }

    #[test]
    fn a_split_crlf_is_one_line_and_a_later_chunk_extends_the_bar() {
        let (crlf, _) = lines_from_pieces(vec![b"hello\r", b"\nnext\n"]);
        let together = lines_from(b"hello\r\nnext\n");
        let one_line =
            crlf == together && crlf.len() == 2 && crlf[0] == "hello" && crlf[1] == "next";
        assert!(one_line);
        let no_blank = crlf
            .iter()
            .all(|line| !line.is_empty() && !line.contains('\r'));
        assert!(no_blank);

        let (extended, seen) = lines_from_pieces(vec![b"\rbar", b"-two"]);
        let grew = extended.len() == 1 && extended[0] == "bar-two";
        assert!(grew);
        let showed_early = seen
            .iter()
            .any(|update| matches!(update, LogUpdate::Revise(text) if text == "bar"));
        assert!(showed_early);
        let not_only_the_suffix = extended.iter().all(|line| line != "-two");
        assert!(not_only_the_suffix);
    }

    #[test]
    fn a_long_piece_without_a_newline_is_capped_and_a_lone_cr_is_nothing() {
        let mut bytes = vec![b'a'; PARTIAL_CAP + 10];
        let lines = lines_from(&bytes);
        let capped = lines.len() == 2 && lines[0].len() == PARTIAL_CAP && lines[1].len() == 10;
        assert!(capped);
        let letters = lines
            .iter()
            .all(|line| line.bytes().all(|byte| byte == b'a'));
        assert!(letters);
        bytes.clear();
        let lone = lines_from(b"\r");
        assert!(lone.is_empty());
    }

    #[test]
    fn an_io_error_drops_the_unfinished_piece() {
        struct ThenBoom {
            sent: bool,
        }
        impl std::io::Read for ThenBoom {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                if !self.sent {
                    self.sent = true;
                    let first = b"ok\npartial";
                    let count = first.len().min(out.len());
                    out[..count].copy_from_slice(&first[..count]);
                    return Ok(count);
                }
                Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "boom"))
            }
        }
        let mut reader = std::io::BufReader::new(ThenBoom { sent: false });
        let mut pump = LogPump::new();
        let mut log = VecDeque::new();
        let mut open = false;
        while let Some(update) = pump.next(&mut reader) {
            apply_log_update(&mut log, &mut open, update);
        }
        let kept = log.len() == 1 && log.front().map(String::as_str) == Some("ok");
        assert!(kept);
    }

    #[test]
    fn repeated_revisions_collapse_in_the_queue() {
        let lines = Mutex::new(VecDeque::new());
        push_shared(&lines, LogUpdate::Commit("banner".to_string()));
        push_shared(&lines, LogUpdate::Revise("one".to_string()));
        push_shared(&lines, LogUpdate::Revise("two".to_string()));
        let guard = lines.lock().unwrap_or_else(|poison| poison.into_inner());
        let collapsed = guard.len() == 2
            && matches!(guard.front(), Some(LogUpdate::Commit(text)) if text == "banner")
            && matches!(guard.back(), Some(LogUpdate::Revise(text)) if text == "two");
        assert!(collapsed);
    }

    fn settled_backprop() -> Option<PathBuf> {
        let started = std::time::Instant::now();
        loop {
            match tool_answer() {
                ToolAnswer::Ready(path) => return path,
                ToolAnswer::Pending => {
                    assert!(started.elapsed() < std::time::Duration::from_secs(8));
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
    }

    #[test]
    fn the_tool_lookup_is_stable_until_the_cache_is_cleared() {
        let first = settled_backprop();
        let second = settled_backprop();
        assert!(first == second);
        bust_tool_cache();
        let _third = tool_answer();
    }

    #[test]
    fn the_installed_lookup_is_an_exe_or_absent() {
        let found = settled_backprop();
        let name = file_name(found.as_deref());
        assert!(
            name.is_none()
                || name == Some("backprop.exe")
                || name == Some("backprop.com")
                || name == Some("backprop")
        );
    }

    #[test]
    fn a_missing_program_is_a_fixed_sentence() {
        let result = start_installed(LaunchRequest {
            program: PathBuf::from("runforge-missing-backprop"),
            args: vec!["train".to_string()],
            cwd: std::env::temp_dir(),
        });
        assert!(matches!(result, Err(START_FAILED)));
    }

    #[cfg(windows)]
    fn system_tool(name: &str) -> PathBuf {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        PathBuf::from(root).join("System32").join(name)
    }

    #[cfg(windows)]
    #[test]
    fn a_short_command_copies_its_log() {
        let mut session = match start_installed(LaunchRequest {
            program: system_tool("where.exe"),
            args: Vec::new(),
            cwd: std::env::temp_dir(),
        }) {
            Ok(session) => session,
            Err(_) => panic!("where"),
        };
        let mut logged = String::new();
        let mut code = None;
        for _ in 0..50 {
            for update in session.take_lines() {
                let text = match update {
                    LogUpdate::Commit(text) | LogUpdate::Revise(text) => text,
                };
                logged.push_str(&text);
            }
            if code.is_none() {
                code = session.finished();
            }
            if code.is_some() && logged.contains("WHERE") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let copied = code.is_some() && logged.contains("WHERE");
        assert!(copied);
    }

    #[cfg(windows)]
    #[test]
    fn stop_ends_ping() {
        let mut session = match spawn_session(LaunchRequest {
            program: system_tool("ping.exe"),
            args: vec!["-n".to_string(), "30".to_string(), "127.0.0.1".to_string()],
            cwd: std::env::temp_dir(),
        }) {
            Ok(session) => session,
            Err(_) => panic!("ping"),
        };
        assert!(session.job.is_some());
        let mut logged = String::new();
        for _ in 0..40 {
            for update in session.take_lines() {
                let text = match update {
                    LogUpdate::Commit(text) | LogUpdate::Revise(text) => text,
                };
                logged.push_str(&text);
            }
            if logged.to_ascii_lowercase().contains("ping") && session.finished().is_none() {
                break;
            }
            if session.finished().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let alive = session.finished().is_none() && logged.to_ascii_lowercase().contains("ping");
        assert!(alive);
        session.stop();
        let mut ended = false;
        for _ in 0..80 {
            if session.finished().is_some() {
                ended = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(ended);
    }

    #[cfg(all(windows, target_pointer_width = "64"))]
    #[test]
    fn the_job_limit_matches_the_os_layout() {
        assert_eq!(size_of::<super::JobBasicLimit>(), 64);
        assert_eq!(std::mem::offset_of!(super::JobBasicLimit, limit_flags), 16);
        assert_eq!(
            std::mem::offset_of!(super::JobBasicLimit, minimum_working_set_size),
            24
        );
        assert_eq!(size_of::<super::JobExtendedLimit>(), 144);
    }

    #[test]
    fn console_control_sequences_do_not_stay_in_the_line() {
        let lines = lines_from(b"\x1b[?25hbanner\x1b[0m\n");
        let clean = lines.len() == 1 && lines.first().map(String::as_str) == Some("banner");
        assert!(clean);
    }

    struct ClearDelay;

    impl Drop for ClearDelay {
        fn drop(&mut self) {
            super::STAT_DELAY_MS.store(0, Ordering::Release);
        }
    }

    fn wait_ready_locked() -> Option<PathBuf> {
        let started = std::time::Instant::now();
        loop {
            match super::tool_answer_locked() {
                ToolAnswer::Ready(path) => return path,
                ToolAnswer::Pending => {
                    assert!(started.elapsed() < std::time::Duration::from_secs(8));
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
    }

    #[test]
    fn an_expired_lookup_does_not_stat_on_the_caller() {
        let _gate = super::cache_gate();
        let _clear = ClearDelay;
        let first = wait_ready_locked();
        super::expire_tool_cache_for_test();
        super::stat_threads()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clear();
        super::STAT_DELAY_MS.store(300, Ordering::Release);
        let began = std::time::Instant::now();
        let again = super::tool_answer_locked();
        let quick = began.elapsed() < std::time::Duration::from_millis(80);
        let same = matches!(again, ToolAnswer::Ready(path) if path == first);
        assert!(quick && same);
        let caller = std::thread::current().id();
        let mut off_thread = false;
        for _ in 0..50 {
            let ids = super::stat_threads()
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .clone();
            if ids.iter().any(|id| *id != caller) {
                off_thread = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(off_thread);
    }

    #[test]
    fn a_cleared_lookup_stays_pending_until_the_walk_finishes() {
        let _gate = super::cache_gate();
        let _clear = ClearDelay;
        let _settled = wait_ready_locked();
        super::stat_threads()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clear();
        super::STAT_DELAY_MS.store(250, Ordering::Release);
        super::bust_tool_cache_locked();
        let began = std::time::Instant::now();
        let answer = super::tool_answer_locked();
        let pending_now = began.elapsed() < std::time::Duration::from_millis(80)
            && matches!(answer, ToolAnswer::Pending);
        assert!(pending_now);
        let caller = std::thread::current().id();
        let mut ready = false;
        let mut off_thread = false;
        for _ in 0..50 {
            let ids = super::stat_threads()
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .clone();
            if ids.iter().any(|id| *id != caller) {
                off_thread = true;
            }
            if matches!(super::tool_answer_locked(), ToolAnswer::Ready(_)) {
                ready = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(ready && off_thread);
    }

    #[cfg(windows)]
    #[test]
    fn a_program_path_with_spaces_is_quoted_on_the_command_line() {
        let line = super::command_line_string(
            Path::new("C:\\Program Files\\backprop.exe"),
            &["train".to_string(), "my file".to_string()],
        );
        let quoted = line == "\"C:\\Program Files\\backprop.exe\" train \"my file\""
            && !line.contains("cmd.exe")
            && !line.contains("PYTHONUNBUFFERED");
        assert!(quoted);
    }

    #[cfg(all(windows, target_pointer_width = "64"))]
    #[test]
    fn the_pseudoconsole_startup_matches_the_os_layout() {
        let startup = size_of::<super::StartupInfoW>() == 104
            && std::mem::offset_of!(super::StartupInfoW, cb) == 0
            && std::mem::offset_of!(super::StartupInfoW, std_input) == 80
            && std::mem::offset_of!(super::StartupInfoW, std_output) == 88
            && std::mem::offset_of!(super::StartupInfoW, std_error) == 96
            && size_of::<super::StartupInfoExW>() == 112
            && size_of::<super::ProcessInformation>() == 24
            && size_of::<super::SecurityAttributes>() == 24
            && size_of::<super::Coord>() == 4;
        assert!(startup);
    }

    #[cfg(windows)]
    #[test]
    fn a_console_banner_shows_before_the_process_exits() {
        let dir = scratch("banner-child");
        let source = dir.join("child.rs");
        std::fs::write(
            &source,
            "fn main() {\n    println!(\"banner\");\n    std::thread::sleep(std::time::Duration::from_secs(30));\n}\n",
        )
        .unwrap();
        let exe = dir.join("child.exe");
        let rustc = PathBuf::from(env!("CARGO")).with_file_name("rustc.exe");
        let compiled = std::process::Command::new(rustc)
            .arg(&source)
            .arg("-o")
            .arg(&exe)
            .status();
        let Ok(status) = compiled else {
            panic!("rustc");
        };
        assert!(status.success());
        let mut session = match spawn_session(LaunchRequest {
            program: exe,
            args: Vec::new(),
            cwd: dir,
        }) {
            Ok(session) => session,
            Err(_) => panic!("spawn"),
        };
        let mut saw = false;
        let mut still_running = false;
        for _ in 0..100 {
            for update in session.take_lines() {
                let text = match update {
                    LogUpdate::Commit(text) | LogUpdate::Revise(text) => text,
                };
                if text == "banner" {
                    saw = true;
                }
            }
            if saw && session.finished().is_none() {
                still_running = true;
                break;
            }
            if session.finished().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(saw && still_running);
        session.stop();
    }
}
