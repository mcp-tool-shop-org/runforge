//! Start an already-installed `backprop`. Arguments are a list. Nothing goes through a shell.
//!
//! `.cmd` and `.bat` are skipped: Windows would run those through `cmd.exe`. Stop kills the
//! process tree with a job object. `std::process::Child` has no suspended thread to resume, so
//! the job is assigned immediately after spawn.

use std::collections::VecDeque;
use std::ffi::c_void;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
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
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x2000;
/// `KILL_ON_JOB_CLOSE` is rejected on the basic limit class (`ERROR_INVALID_PARAMETER`).
const JOB_EXTENDED_LIMIT: i32 = 9;

pub(crate) struct LaunchRequest {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

pub(crate) trait Session {
    fn take_lines(&mut self) -> Vec<String>;
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
            if candidate.is_absolute() && candidate.is_file() {
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

struct ToolHit {
    at: Instant,
    path: Option<PathBuf>,
}

fn tool_cache() -> &'static Mutex<Option<ToolHit>> {
    static CACHE: Mutex<Option<ToolHit>> = Mutex::new(None);
    &CACHE
}

/// Drop the cached hit or miss so the next lookup walks `PATH` again.
pub(crate) fn bust_tool_cache() {
    let mut guard = tool_cache()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    *guard = None;
}

/// A hit or a miss is reused for two seconds so a frame does not stat `PATH`.
pub(crate) fn installed_backprop() -> Option<PathBuf> {
    let mut guard = tool_cache()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    if let Some(hit) = guard.as_ref()
        && hit.at.elapsed() < Duration::from_secs(2)
    {
        return hit.path.clone();
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    let pathext = std::env::var_os("PATHEXT").unwrap_or_default();
    let found = find_backprop(&path.to_string_lossy(), &pathext.to_string_lossy());
    *guard = Some(ToolHit {
        at: Instant::now(),
        path: found.clone(),
    });
    found
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

pub(crate) fn start_installed(request: LaunchRequest) -> Result<Box<dyn Session>, &'static str> {
    Ok(Box::new(spawn_session(request)?))
}

pub(crate) fn spawn_session(request: LaunchRequest) -> Result<LiveSession, &'static str> {
    let mut command = Command::new(&request.program);
    command
        .args(&request.args)
        .current_dir(&request.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|_| START_FAILED)?;
    #[cfg(windows)]
    let job = assign_kill_on_close(&child);
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
        #[cfg(windows)]
        job,
    })
}

/// One log line, or `None` at EOF or on an IO error. A non-UTF-8 byte stays in the line.
fn next_log_line(reader: &mut impl BufRead) -> Option<String> {
    let mut chunk = Vec::new();
    match reader.read_until(b'\n', &mut chunk) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(lossy_log_line(&chunk)),
    }
}

fn lossy_log_line(bytes: &[u8]) -> String {
    let mut end = bytes.len();
    if bytes.last() == Some(&b'\n') {
        end -= 1;
    }
    if end > 0 && bytes[end - 1] == b'\r' {
        end -= 1;
    }
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn spawn_reader<R>(
    reader: R,
    lines: &Arc<Mutex<VecDeque<String>>>,
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
        while let Some(line) = next_log_line(&mut buffered) {
            push_shared(&lines, line);
        }
        readers_left.fetch_sub(1, Ordering::Relaxed);
    })
}

fn push_shared(lines: &Mutex<VecDeque<String>>, line: String) {
    let mut guard = lines.lock().unwrap_or_else(|poison| poison.into_inner());
    remember_line(&mut guard, line);
}

pub(crate) struct LiveSession {
    child: Option<Child>,
    lines: Arc<Mutex<VecDeque<String>>>,
    readers: Vec<thread::JoinHandle<()>>,
    readers_left: Arc<AtomicUsize>,
    exit_code: Option<i32>,
    #[cfg(windows)]
    job: Option<Job>,
}

impl Session for LiveSession {
    fn take_lines(&mut self) -> Vec<String> {
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

    fn stop(&mut self) {
        #[cfg(windows)]
        if let Some(mut job) = self.job.take() {
            job.terminate();
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
        }
    }
}

impl Drop for LiveSession {
    fn drop(&mut self) {
        self.stop();
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
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
fn assign_kill_on_close(child: &Child) -> Option<Job> {
    use std::os::windows::io::AsRawHandle;
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
        if set == 0 || AssignProcessToJobObject(handle, child.as_raw_handle()) == 0 {
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
}

#[cfg(test)]
mod tests {
    use super::{
        BAD_ARGUMENT, BAD_STEPS, LOG_CAP, LaunchRequest, NEED_DATA, NO_CHECKPOINT, OPEN_FOLDER,
        SELECT_RUN, START_FAILED, Session, bust_tool_cache, eval_args, exit_note, export_args,
        find_backprop, installed_backprop, next_log_line, push_shared, remember_line,
        spawn_session, start_installed, train_args,
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
        push_shared(&lines, "kept".to_string());
        let guard = lines.lock().unwrap_or_else(|poison| poison.into_inner());
        assert_eq!(guard.back().map(String::as_str), Some("kept"));
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

    #[test]
    fn a_bad_byte_does_not_drop_the_rest_of_the_log() {
        let bytes = b"ok\n\xff\nnext\n";
        let mut cursor = std::io::Cursor::new(&bytes[..]);
        let mut lines = Vec::new();
        while let Some(line) = next_log_line(&mut cursor) {
            lines.push(line);
        }
        let later = lines.iter().skip(1).any(|line| line == "next");
        assert!(later);
    }

    #[test]
    fn the_tool_lookup_is_stable_until_the_cache_is_cleared() {
        let first = installed_backprop();
        let second = installed_backprop();
        assert!(first == second);
        bust_tool_cache();
        let _third = installed_backprop();
    }

    #[test]
    fn the_installed_lookup_is_an_exe_or_absent() {
        let found = installed_backprop();
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
        let mut lines = 0usize;
        let mut code = None;
        for _ in 0..50 {
            lines += session.take_lines().len();
            if code.is_none() {
                code = session.finished();
            }
            if code.is_some() && lines > 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(code.is_some());
        assert!(lines > 0);
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
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(session.finished().is_none());
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
}
