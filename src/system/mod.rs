//! System resources abstractions, such as operations with the filesystem and
//! processes.

pub fn check_process_running(
    name: &'static std::path::Path,
) -> Result<Option<crate::core::LinuxProcessId>, Error> {
    let processes: procfs::process::ProcessesIter =
        procfs::process::all_processes().map_err(Error::ProcFsError)?;

    let mut matching_pids: Vec<u32> = Vec::new();
    for proc in processes {
        let proc: procfs::process::Process = proc.map_err(Error::ProcFsError)?;
        if let Ok(stat) = proc.stat() {
            let proc_exec_filename: &std::path::Path = std::path::Path::new(&stat.comm);
            if proc_exec_filename == name {
                matching_pids.push(stat.pid.try_into().expect("process ID should be a u32"));
            }
        }
    }

    match matching_pids.len() {
        0 => Ok(None),
        1 => Ok(Some(matching_pids[0])),
        _ => Err(Error::RunningParallel(matching_pids)),
    }
}

#[derive(Debug)]
pub enum Error {
    /// Contains name or path (relative or absolute) of the file that was
    /// not found, and possible associated underlying system IO error.
    FileNotFound((std::path::PathBuf, Option<std::io::Error>)),
    MultipleFilesFound(Vec<std::path::PathBuf>),
    ProcFsError(procfs::ProcError),
    RunningParallel(Vec<u32>),
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::FileNotFound((_, Some(err))) => Some(err),
            Error::FileNotFound((_, None)) => None,
            Error::MultipleFilesFound(_) => None,
            Error::ProcFsError(err) => Some(err),
            Error::RunningParallel(_) => None,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::FileNotFound((file, _)) => {
                write!(f, "file not found: {}", file.to_string_lossy())
            }
            Error::MultipleFilesFound(found) => {
                let found: Vec<String> = found
                    .iter()
                    .map(|n| n.to_string_lossy().into_owned())
                    .collect::<Vec<String>>();
                return write!(
                    f,
                    "unexpected more than 1 files found: {}",
                    found.join(", ")
                );
            }
            Error::ProcFsError(_) => write!(f, "dependency 'procfs' failed"),
            Error::RunningParallel(vec) => {
                write!(
                    f,
                    "parallel processes running: PIDs {}",
                    vec.iter()
                        .map(|n| n.to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
        }
    }
}

pub struct ExistingFile {
    pub file_name: std::path::PathBuf,
    pub absolute_path_file: std::path::PathBuf,
    pub absolute_path_parent: std::path::PathBuf,
    pub last_change: chrono::DateTime<chrono::Utc>,
}
impl ExistingFile {
    pub fn check(path: &std::path::Path) -> Result<Self, Error> {
        let metadata: std::fs::Metadata = match path.metadata() {
            Ok(n) => n,
            Err(err) => return Err(Error::FileNotFound((path.into(), Some(err)))),
        };
        let absolute_path_file: std::path::PathBuf = match path.canonicalize() {
            Ok(n) => n,
            Err(err) => return Err(Error::FileNotFound((path.into(), Some(err)))),
        };
        let absolute_path_parent: std::path::PathBuf = match path.parent() {
            Some(n) => n.into(),
            None => unreachable!("absolute path to a file should have a parent"),
        };
        let file_name: std::path::PathBuf = match absolute_path_file.file_name() {
            Some(n) => std::path::PathBuf::from(n),
            None => unreachable!("absolute path to a file should have a file name"),
        };
        let ctime: i64 = std::os::unix::fs::MetadataExt::ctime(&metadata);
        let last_change: chrono::DateTime<chrono::Utc> =
            match chrono::DateTime::from_timestamp(ctime, 0) {
                Some(n) => n,
                None => {
                    unreachable!("ctime of an existing file should be a valid timestamp")
                }
            };

        return Ok(Self {
            absolute_path_file,
            absolute_path_parent,
            file_name,
            last_change,
        });
    }
}

pub fn find_single_file(
    executable_name: &'static std::path::Path,
    exclude_from_search: Option<std::path::PathBuf>,
) -> Result<Option<ExistingFile>, Error> {
    let mut matches: Vec<std::path::PathBuf> = Vec::new();

    if let None = exclude_from_search {
        log::debug!(
            "Doing a full system wide search for a file named {}... This might take a while",
            executable_name.to_string_lossy()
        );
    }
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

        if entry.file_name() == executable_name && entry.file_type().is_file() {
            matches.push(entry.path().to_path_buf());
        }

        if matches.len() > 1 {
            return Err(Error::MultipleFilesFound(matches));
        }
    }

    match matches.len() {
        0 => Err(Error::FileNotFound((executable_name.into(), None))),
        1 => {
            let path: std::path::PathBuf = matches
                .into_iter()
                .next()
                .expect("iterator of length 1 should have a first next");
            let file: ExistingFile = ExistingFile::check(&path)?;
            return Ok(Some(file));
        }
        _ => unreachable!("iterator should have length 0 or 1 at this point"),
    }
}

pub fn trace_log_child_output_and_wait_to_terminate(
    mut child: std::process::Child,
) -> Result<(String, String, std::process::ExitStatus), std::io::Error> {
    let stdout: Option<std::process::ChildStdout> = child.stdout.take();
    let stderr: Option<std::process::ChildStderr> = child.stderr.take();

    let stdout_thread: std::thread::JoinHandle<String> = std::thread::spawn(move || {
        let mut output: String = String::new();
        if let Some(out) = stdout {
            let reader: std::io::BufReader<std::process::ChildStdout> =
                std::io::BufReader::new(out);
            for line in std::io::BufRead::lines(reader).flatten() {
                log::trace!("{line}");
                output.push_str(&line);
                output.push('\n');
            }
        }
        return output;
    });

    let stderr_thread: std::thread::JoinHandle<String> = std::thread::spawn(move || {
        let mut output: String = String::new();
        if let Some(err) = stderr {
            let reader: std::io::BufReader<std::process::ChildStderr> =
                std::io::BufReader::new(err);
            for line in std::io::BufRead::lines(reader).flatten() {
                log::trace!("{line}");
                output.push_str(&line);
                output.push('\n');
            }
        }
        return output;
    });

    let exit_status: std::process::ExitStatus = child.wait()?;
    let stdout_content: String = stdout_thread.join().unwrap_or_default();
    let stderr_content: String = stderr_thread.join().unwrap_or_default();

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
