pub struct Command {
    cmd: std::process::Command,
    cwd: std::path::PathBuf,
    strace_output_inmem: std::fs::File,
}
impl Command {
    pub fn strace(cwd: &std::path::PathBuf, cmd_vec: Vec<&str>) -> Self {
        /* TODO: Check that strace output parsing works with "-f" like it does
        with "-ff", and only use one of the two everywhere! */

        let mut strace_argv: Vec<&str> = vec![
            /* N.B. Using "-f" instead of "-ff" to make strace write outputs
            into a single (inmem) file, instead of letting it make a new file
            for each spawned subprocess. */
            "-f",
            "-e",
            "trace=file",
        ];

        let (inmem_fd, inmem_path) = Command::make_inmem_file_owned();
        strace_argv.push("-o");
        strace_argv.push(&inmem_path);

        let mut cmd = std::process::Command::new("strace");
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.current_dir(&cwd);
        cmd.args(vec![strace_argv, cmd_vec].concat());
        return Self {
            cmd,
            cwd: cwd.clone(),
            strace_output_inmem: inmem_fd,
        };
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
        std::io::Read::read_to_string(&mut self.strace_output_inmem, &mut strace_output_inmem)
            .unwrap(); // TODO: Don't panic!

        let paths: std::collections::HashSet<String> =
            crate::misc::extract_modified_paths(&strace_output_inmem, &self.cwd);
        let paths: Vec<(String, u64)> = crate::misc::get_sizes(paths);
        return Ok(paths);
    }

    fn make_inmem_file_owned() -> (std::fs::File, String) {
        let inmem_file_name: &std::ffi::CStr = &std::ffi::CString::new("strace_out.inmem").unwrap(); // TODO: Don't panic!

        let inmem_fd: std::os::fd::OwnedFd = nix::sys::memfd::memfd_create(
            inmem_file_name,
            nix::sys::memfd::MemFdCreateFlag::empty(),
        )
        .unwrap(); // TODO: Don't panic!
        let inmem_fd: i32 = std::os::fd::IntoRawFd::into_raw_fd(inmem_fd);

        let inmem_file: std::fs::File = unsafe {
            use std::os::fd::FromRawFd;
            std::fs::File::from_raw_fd(inmem_fd)
        };
        let path: String = format!("/proc/self/fd/{}", inmem_fd.to_string());

        return (inmem_file, path);
    }
}
