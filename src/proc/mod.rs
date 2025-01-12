pub struct Dependency<'env> {
    executable: &'static std::path::Path,
    env: Option<std::collections::HashMap<&'env str, &'env str>>,
}

impl<'env> Dependency<'env> {
    pub fn init(
        executable: &'static std::path::Path,
        env: Option<std::collections::HashMap<&'env str, &'env str>>,
    ) -> Self {
        // TODO: Use sh -c command -v to assure that the given executable dependency exists, and panic if not exists?
        return Self { executable, env };
    }
}

impl<'env> Exec for Dependency<'env> {
    fn exec(
        &self,
        work_dir: Option<&std::path::Path>,
        argv: Vec<&str>,
        stdout_sender: std::sync::mpsc::Sender<String>,
        stderr_sender: std::sync::mpsc::Sender<String>,
        run_till_end: bool,
    ) -> Result<(), ExecError> {
        let mut command: std::process::Command = std::process::Command::new(&self.executable);

        if let Some(env_vars) = &self.env {
            command.envs(env_vars);
        }

        if let Some(dir) = work_dir {
            command.current_dir(&dir);
        }

        command.args(&argv);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let mut child: std::process::Child = match command.spawn() {
            Ok(process) => process,
            Err(err) => {
                return Err(ExecError {
                    cmd_fmted: format!("{} {:?}", &self.executable.display(), &argv),
                    status: None,
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

        if run_till_end {
            let status = match child.wait() {
                Ok(status) => status.code(),
                Err(_) => todo!(),
            };

            let _ = stdout_thread.join();
            let _ = stderr_thread.join();

            if status != Some(0) {
                return Err(ExecError {
                    cmd_fmted: format!("{} {:?}", self.executable.display(), argv),
                    status,
                });
            }
        }

        return Ok(());
    }
}

struct ExecError {
    /// Executable and its argument vector.
    cmd_fmted: String,
    /// The numeric code with which the execution terminated.
    status: Option<i32>, // TODO: i32 or whatever?
}
// TODO: impl std::error::Error for ExecError

trait Exec {
    fn exec(
        &self,
        work_dir: Option<&std::path::Path>,
        argv: Vec<&str>,
        stdout_sender: std::sync::mpsc::Sender<String>,
        stderr_sender: std::sync::mpsc::Sender<String>,
        run_till_end: bool,
    ) -> Result<(), ExecError>;
}

#[deprecated = "Use `impl Exec for Dependency` instead"]
pub struct Command {
    cmd: std::process::Command,
    cwd: std::path::PathBuf,
    strace_output_inmem: std::fs::File,
}
impl Command {
    pub fn strace(
        cwd: &std::path::PathBuf,
        cmd_vec: Vec<&str>,
    ) -> Result<Self, crate::error::FatalError> {
        let mut strace_argv: Vec<&str> = vec![
            /* N.B. Using "-f" instead of "-ff" to make strace write outputs
            into a single (inmem) file, instead of letting it make a new file
            for each spawned subprocess. */
            "-f",
            "-e",
            "trace=file",
        ];

        let (inmem_fd, inmem_path) = Command::make_inmem_file_owned()?;
        strace_argv.push("-o");
        strace_argv.push(&inmem_path);

        let mut cmd = std::process::Command::new("strace");
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.current_dir(&cwd);
        cmd.args(vec![strace_argv, cmd_vec].concat());
        return Ok(Self {
            cmd,
            cwd: cwd.clone(),
            strace_output_inmem: inmem_fd,
        });
    }

    pub fn run_to_end(&mut self) -> Result<Vec<(String, u64)>, crate::error::FatalError> {
        let out: std::process::Output = match self.cmd.output() {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!("cannot execute: {:?}", self.cmd),
                    Some(Box::new(err)),
                ));
            }
        };
        if !out.status.success() {
            let stderr: String = match String::from_utf8(out.stderr) {
                Ok(n) => n,
                Err(err) => {
                    return Err(crate::error::FatalError::new(
                        format!("cannot decode STDERR of command as UTF-8: {:?}", self.cmd),
                        Some(Box::new(err)),
                    ));
                }
            };
            return Err(crate::error::FatalError::new(
                format!(
                    "command exited with unsuccessful status: {:?}: STDERR:\n{}",
                    self.cmd, stderr
                ),
                None,
            ));
        }

        let mut strace_output_inmem: String = String::new();
        let read: Result<usize, std::io::Error> =
            std::io::Read::read_to_string(&mut self.strace_output_inmem, &mut strace_output_inmem);

        if let Err(err) = read {
            return Err(crate::error::FatalError::new(
                format!("could not read output of strace: {:?}", self.cmd),
                Some(Box::new(err)),
            ));
        }

        let paths: std::collections::HashSet<String> =
            crate::misc::extract_modified_paths(&strace_output_inmem, &self.cwd);
        let paths: Vec<(String, u64)> = crate::misc::get_sizes(paths);
        return Ok(paths);
    }

    fn make_inmem_file_owned() -> Result<(std::fs::File, String), crate::error::FatalError> {
        let inmem_file_name: std::ffi::CString = match std::ffi::CString::new("strace_out.inmem") {
            Ok(n) => n,
            Err(_) => {
                /* Constructing a "C string" from a static immutable &str should
                either always succeed or never succeed. */
                unreachable!();
            }
        };

        let inmem_fd: std::os::fd::OwnedFd = match nix::sys::memfd::memfd_create(
            &inmem_file_name,
            nix::sys::memfd::MemFdCreateFlag::empty(),
        ) {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!("cannot create in-mem file"),
                    Some(Box::new(err)),
                ))
            }
        };
        let inmem_fd: i32 = std::os::fd::IntoRawFd::into_raw_fd(inmem_fd);

        let inmem_file: std::fs::File = unsafe {
            use std::os::fd::FromRawFd;
            std::fs::File::from_raw_fd(inmem_fd)
        };
        let path: String = format!("/proc/self/fd/{}", inmem_fd.to_string());

        return Ok((inmem_file, path));
    }
}
