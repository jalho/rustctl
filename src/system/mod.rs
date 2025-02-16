//! System resources abstractions, such as operations with the filesystem and
//! processes.

enum IdentifySingleProcessError {
    LibProcfsFailure { lib_error: procfs::ProcError },
    RunningParallel { pids_found: Vec<u32> },
}
pub fn check_process_running(
    name: &'static std::path::Path,
) -> Result<Option<crate::core::LinuxProcessId>, IdentifySingleProcessError> {
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

pub enum FindSingleFileError {
    FileNotFound {
        filename_seeked: std::path::PathBuf,
    },
    ManyFilesFound {
        paths_absolute_found: Vec<std::path::PathBuf>,
    },
}

pub fn find_single_file(
    seekable_file_name: &std::path::Path,
    exclude_from_search: Option<std::path::PathBuf>,
) -> Result<ExistingFile, FindSingleFileError> {
    let mut matches: Vec<std::path::PathBuf> = Vec::new();

    if let None = exclude_from_search {
        log::debug!(
            "Doing a full system wide search for a file named {}... This might take a while",
            seekable_file_name.to_string_lossy()
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
        }),
        1 => {
            let path: std::path::PathBuf = matches
                .into_iter()
                .next()
                .expect("iterator of length 1 should have a first next");
            let file: ExistingFile = ExistingFile::check(&path)?;
            return Ok(file);
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
