//! System resources abstractions, such as operations with the filesystem and
//! processes.

use std::os::unix::fs::MetadataExt;

#[derive(Debug)]
pub enum IdentifySingleProcessError {
    LibProcfsFailure { lib_error: procfs::ProcError },
    RunningParallel { pids_found: Vec<u32> },
}

impl std::error::Error for IdentifySingleProcessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IdentifySingleProcessError::LibProcfsFailure { lib_error } => Some(lib_error),
            IdentifySingleProcessError::RunningParallel { pids_found: _ } => None,
        }
    }
}

impl std::fmt::Display for IdentifySingleProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentifySingleProcessError::LibProcfsFailure { lib_error: _ } => {
                write!(f, "dependency failed")
            }
            IdentifySingleProcessError::RunningParallel { pids_found } => write!(
                f,
                "running parallel: {} processes found: {}",
                pids_found.len(),
                pids_found
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

pub fn check_process_running(
    name: &std::path::Path,
) -> Result<Option<u32>, IdentifySingleProcessError> {
    log::debug!(
        "Searching for processes named {}...",
        name.to_string_lossy()
    );
    let processes: procfs::process::ProcessesIter = match procfs::process::all_processes() {
        Ok(n) => n,
        Err(err) => return Err(IdentifySingleProcessError::LibProcfsFailure { lib_error: err }),
    };

    let mut matching_pids: Vec<u32> = Vec::new();
    for proc in processes {
        match proc {
            Ok(proc) => {
                if let Ok(stat) = proc.stat() {
                    let proc_exec_filename: &std::path::Path = std::path::Path::new(&stat.comm);
                    if proc_exec_filename == name {
                        matching_pids
                            .push(stat.pid.try_into().expect("process ID should be a u32"));
                    }
                }
            }
            Err(err) => {
                return Err(IdentifySingleProcessError::LibProcfsFailure { lib_error: err })
            }
        }
    }

    match matching_pids.len() {
        0 => Ok(None),
        1 => Ok(Some(matching_pids[0])),
        _ => Err(IdentifySingleProcessError::RunningParallel {
            pids_found: matching_pids,
        }),
    }
}

#[derive(Debug)]
pub enum FindSingleFileError {
    FileNotFound {
        filename_seeked: std::path::PathBuf,
        system_error: Option<std::io::Error>,
    },
    ManyFilesFound {
        paths_absolute_found: Vec<std::path::PathBuf>,
    },
}

pub struct FoundFile {
    pub dir_path_absolute: std::path::PathBuf,
    pub filename: std::path::PathBuf,
    pub last_modified: chrono::DateTime<chrono::Utc>,
    pub metadata: std::fs::Metadata,
}

impl FoundFile {
    pub fn get_absolute_path(&self) -> std::path::PathBuf {
        let mut absolute_path: std::path::PathBuf = self.dir_path_absolute.clone();
        absolute_path.push(&self.filename);
        return absolute_path;
    }
}

impl std::fmt::Display for FoundFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_absolute_path().to_string_lossy())
    }
}

pub fn find_single_file(
    seekable_file_name: &std::path::Path,
    exclude_from_search: Option<&std::path::Path>,
) -> Result<FoundFile, FindSingleFileError> {
    let mut matches: Vec<std::path::PathBuf> = Vec::new();
    log::debug!("Searching for {}...", seekable_file_name.to_string_lossy());
    for entry in walkdir::WalkDir::new("/")
        .into_iter()
        .filter_entry(|e| {
            if let Some(ref exclude_path) = exclude_from_search {
                if e.path().starts_with(exclude_path) {
                    return false;
                }
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        let entry: walkdir::DirEntry = entry;

        if entry.file_name() == seekable_file_name && entry.file_type().is_file() {
            matches.push(entry.path().to_path_buf());
        }

        if matches.len() > 1 {
            return Err(FindSingleFileError::ManyFilesFound {
                paths_absolute_found: matches,
            });
        }
    }

    match matches.len() {
        0 => Err(FindSingleFileError::FileNotFound {
            filename_seeked: seekable_file_name.to_path_buf(),
            system_error: None,
        }),
        1 => {
            let path: std::path::PathBuf = matches
                .into_iter()
                .next()
                .expect("iterator of length 1 should have a first next");
            let file: std::path::PathBuf = match path.canonicalize() {
                Ok(n) => n,
                Err(err) => {
                    return Err(FindSingleFileError::FileNotFound {
                        filename_seeked: seekable_file_name.to_path_buf(),
                        system_error: Some(err),
                    })
                }
            };
            let metadata = file.metadata().expect("existing file should have metadata");

            return Ok(FoundFile {
                dir_path_absolute: file
                    .parent()
                    .expect("absolute path of an existing file should have parent")
                    .to_path_buf(),
                filename: file
                    .file_name()
                    .expect("existing file should have name")
                    .into(),
                last_modified: chrono::DateTime::<chrono::Utc>::from_timestamp(metadata.mtime(), 0)
                    .expect("existing file should have mtime"),
                metadata,
            });
        }
        _ => unreachable!("iterator should have length 0 or 1 at this point"),
    }
}

/// In a new, named Linux thread, read lines to a new buffer.
fn read_lines_to_buf_in_named_thread<R: std::io::Read + Send + 'static>(
    readable: R,
    thread_name: &'static str,
) -> String {
    let th = std::thread::Builder::new().name(thread_name.into());
    let th_attempt = th.spawn(move || {
        let mut content: String = String::new();
        let reader: std::io::BufReader<R> = std::io::BufReader::new(readable);
        for line in std::io::BufRead::lines(reader).flatten() {
            log::trace!("{line}");
            content.push_str(&line);
            content.push('\n');
        }
        return content;
    });

    match th_attempt {
        Ok(th_handle) => match th_handle.join() {
            Ok(content) => return content,
            Err(_) => return String::new(),
        },
        Err(_) => return String::new(),
    };
}

/// Log and collect given childs process's outputs (STDOUT and STDERR), and wait
/// for it to terminate.
pub fn trace_log_child_output_and_wait_to_terminate(
    mut child: std::process::Child,
) -> Result<(String, String, std::process::ExitStatus), std::io::Error> {
    let stdout_content: String = match child.stdout.take() {
        Some(stdout) => read_lines_to_buf_in_named_thread(stdout, "stdout"),
        None => String::new(),
    };
    let stderr_content: String = match child.stderr.take() {
        Some(stderr) => read_lines_to_buf_in_named_thread(stderr, "stderr"),
        None => String::new(),
    };

    let exit_status: std::process::ExitStatus = child.wait()?;

    return Ok((stdout_content, stderr_content, exit_status));
}

pub fn trace_log_child_output(
    child: &mut std::process::Child,
) -> Result<(std::thread::JoinHandle<()>, std::thread::JoinHandle<()>), std::io::Error> {
    let stdout: Option<std::process::ChildStdout> = child.stdout.take();
    let stderr: Option<std::process::ChildStderr> = child.stderr.take();

    let stdout_thread: std::thread::JoinHandle<()> = std::thread::spawn(move || {
        if let Some(n) = stdout {
            let reader: std::io::BufReader<std::process::ChildStdout> = std::io::BufReader::new(n);
            for line in std::io::BufRead::lines(reader).flatten() {
                log::trace!("{line}");
            }
        }
    });

    let stderr_thread: std::thread::JoinHandle<()> = std::thread::spawn(move || {
        if let Some(n) = stderr {
            let reader: std::io::BufReader<std::process::ChildStderr> = std::io::BufReader::new(n);
            for line in std::io::BufRead::lines(reader).flatten() {
                log::trace!("{line}");
            }
        }
    });

    return Ok((stdout_thread, stderr_thread));
}
