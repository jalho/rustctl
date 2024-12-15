pub struct Command {
    cmd: std::process::Command,
    cwd: std::path::PathBuf,
}
impl Command {
    pub fn strace(cwd: &std::path::PathBuf, cmd_vec: Vec<&str>) -> Self {
        let strace_argv: Vec<&str> = vec!["-ff", "-e", "trace=file"];
        let mut cmd = std::process::Command::new("strace");
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.current_dir(&cwd);
        cmd.args(vec![strace_argv, cmd_vec].concat());
        return Self {
            cmd,
            cwd: cwd.clone(),
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
            return Err(crate::error::FatalError::new(
                format!("command exited with unsuccessful status: {:?}", self.cmd),
                None,
            ));
        }
        let stderr: String = match String::from_utf8(out.stderr) {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!("cannot decode output as UTF-8 of command: {:?}", self.cmd),
                    Some(Box::new(err)),
                ));
            }
        };
        let paths: std::collections::HashSet<String> =
            crate::misc::extract_modified_paths(&stderr, &self.cwd);
        let paths: Vec<(String, u64)> = crate::misc::get_sizes(paths);
        return Ok(paths);
    }
}
