pub struct Dependency {
    executable: &'static std::path::Path,
    env: Option<std::collections::HashMap<&str, &str>>,
}

impl Dependency {
    pub fn init(
        executable: &'static std::path::Path,
        env: Option<std::collections::HashMap<&str, &str>>,
    ) -> Self {
        // TODO: Use sh -c command -v to assure that the given executable dependency exists, and panic if not exists?
        return Self { executable, env };
    }
}

impl Exec for Dependency {
    fn exec(
        work_dir: Option<&std::path::Path>,
        argv: Vec<&str>,
        stdout: Option<std::sync::mpsc::Sender<String>>,
        stderr: Option<std::sync::mpsc::Sender<String>>,
    ) -> Result<ExecSuccess, ExecError> {
        // TODO: Run the command to finish if no Senders are given, otherwise just spawn and send output via Senders line by line as it appears
        // TODO: Consider termination without status and status != 0 an error case
    }
}

struct ExecError {
    /// Executable and its argument vector.
    cmd_fmted: String,
    /// The numeric code with which the execution terminated.
    status: Option<i32>, // TODO: i32 or whatever?
    /// STDERR from the execution, if any.
    stderr_utf8: Option<String>,
}

struct ExecSuccess {
    /// STDOUT from the execution, if any.
    stdout_utf8: Option<String>,
}

trait Exec {
    fn exec(
        work_dir: Option<&std::path::Path>,
        argv: Vec<&str>,
        env: Option<std::collections::HashMap<&str, &str>>,
        stdout: Option<std::sync::mpsc::Sender<String>>,
        stderr: Option<std::sync::mpsc::Sender<String>>,
    ) -> Result<ExecSuccess, ExecError>;
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
