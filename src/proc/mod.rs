//! Abstractions related to handling processes on the system.

use crate::misc::is_writeable_dir;

pub struct Dependency {
    pub executable: &'static str,
    work_dir: std::path::PathBuf,
}

impl Dependency {
    pub fn init(
        executable: &'static str,
        work_dir: &std::path::Path,
    ) -> Result<Self, crate::error::ErrPrecondition> {
        let sh: &str = "sh";
        let output: std::process::Output = match std::process::Command::new(sh)
            .arg("-c")
            .arg(format!("command -v {}", executable))
            .output()
        {
            Ok(n) => n,
            Err(_) => {
                return Err(crate::error::ErrPrecondition::MissingDependency(
                    sh.to_owned(),
                ));
            }
        };

        if !output.status.success() {
            return Err(crate::error::ErrPrecondition::MissingDependency(
                executable.to_owned(),
            ));
        }

        let work_dir: std::path::PathBuf = work_dir.into();
        if !is_writeable_dir(&work_dir) {
            return Err(crate::error::ErrPrecondition::MissingPermission(format!(
                "rwx to {}",
                &work_dir.to_string_lossy()
            )));
        }

        return Ok(Self {
            executable,
            work_dir,
        });
    }
}

pub trait Exec {
    fn exec_terminating(&self, argv: Vec<&str>) -> Result<(String, String), crate::error::ErrExec>;

    fn exec_continuous(
        &self,
        argv: Vec<&str>,
        stdout_sender: std::sync::mpsc::Sender<String>,
        stderr_sender: std::sync::mpsc::Sender<String>,
    ) -> Result<(std::thread::JoinHandle<()>, std::thread::JoinHandle<()>), crate::error::ErrExec>;
}

impl Exec for Dependency {
    fn exec_continuous(
        &self,
        argv: Vec<&str>,
        stdout_sender: std::sync::mpsc::Sender<String>,
        stderr_sender: std::sync::mpsc::Sender<String>,
    ) -> Result<(std::thread::JoinHandle<()>, std::thread::JoinHandle<()>), crate::error::ErrExec>
    {
        let mut command: std::process::Command = std::process::Command::new(&self.executable);

        command.current_dir(&self.work_dir);
        command.args(&argv);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let mut child: std::process::Child = match command.spawn() {
            Ok(process) => process,
            Err(err) => {
                return Err(crate::error::ErrExec {
                    command: format!("{} {:?}", &self.executable, &argv),
                    status: None,
                    stderr: None,
                });
            }
        };

        let stdout: std::process::ChildStdout =
            child.stdout.take().expect("Failed to capture stdout"); // TODO: Don't panic!
        let stderr: std::process::ChildStderr =
            child.stderr.take().expect("Failed to capture stderr"); // TODO: Don't panic!

        let stdout_thread: std::thread::JoinHandle<()> = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in std::io::BufRead::lines(reader) {
                if let Ok(line) = line {
                    let _ = stdout_sender.send(line);
                }
            }
        });

        let stderr_thread: std::thread::JoinHandle<()> = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            for line in std::io::BufRead::lines(reader) {
                if let Ok(line) = line {
                    let _ = stderr_sender.send(line);
                }
            }
        });

        return Ok((stdout_thread, stderr_thread));
    }

    fn exec_terminating(&self, argv: Vec<&str>) -> Result<(String, String), crate::error::ErrExec> {
        let mut command: std::process::Command = std::process::Command::new(&self.executable);

        command.current_dir(&self.work_dir);
        command.args(&argv);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let output: std::process::Output = match command.output() {
            Ok(n) => n,
            Err(_) => {
                return Err(crate::error::ErrExec {
                    command: format!("{} {:?}", &self.executable, &argv),
                    status: None,
                    stderr: None,
                });
            }
        };

        let stdout: String = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr: String = String::from_utf8_lossy(&output.stderr).into_owned();

        if !output.status.success() {
            return Err(crate::error::ErrExec {
                command: format!("{} {:?}", &self.executable, &argv),
                stderr: Some(stderr),
                status: output.status.code(),
            });
        }

        return Ok((stdout, stderr));
    }
}

type ProcessId = u32;
/// Check whether a given process is already running.
pub fn is_process_running(name_seekable: &str) -> Option<ProcessId> {
    let name_seekable: &std::path::Path = std::path::Path::new(&name_seekable);
    let name_seekable: &std::ffi::OsStr = match name_seekable.file_name() {
        Some(n) => n,
        None => return None,
    };
    let name_seekable: String = match name_seekable.to_str() {
        Some(n) => n.to_owned(),
        None => return None,
    };

    let proc_dir: &str = "/proc/";
    let dir: std::fs::ReadDir = match std::fs::read_dir(proc_dir) {
        Ok(n) => n,
        Err(_) => unreachable!("{proc_dir} should always exist"),
    };

    for entry in dir {
        let entry: std::fs::DirEntry = match entry {
            Ok(n) => n,
            Err(_) => continue,
        };
        let path: std::path::PathBuf = entry.path();
        if !path.is_dir() {
            continue;
        }

        let filename: &std::ffi::OsStr = match path.file_name() {
            Some(n) => n,
            None => continue,
        };

        let filename: &str = match filename.to_str() {
            Some(n) => n,
            None => continue,
        };

        let pid: u32 = match filename.parse::<u32>() {
            Ok(n) => n,
            Err(_) => continue,
        };

        let path: std::path::PathBuf = path.join("comm");

        let proc_name: String = match std::fs::read_to_string(&path) {
            Ok(n) => n.trim().to_owned(),
            Err(_) => continue,
        };

        if proc_name == name_seekable {
            return Some(pid);
        }
    }
    return None;
}
