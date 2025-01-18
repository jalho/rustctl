//! Abstractions related to handling processes on the system.

pub struct Dependency {
    executable: &'static str,
}

impl Dependency {
    pub fn init(executable: &'static str) -> Self {
        // TODO: Use sh -c command -v to assure that the given executable dependency exists, and panic if not exists?
        return Self { executable };
    }
}

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

impl Exec for Dependency {
    fn exec(
        &self,
        work_dir: Option<&std::path::Path>,
        argv: Vec<&str>,
        stdout_sender: std::sync::mpsc::Sender<String>,
        stderr_sender: std::sync::mpsc::Sender<String>,
        run_till_end: bool,
    ) -> Result<(), ExecError> {
        let mut command: std::process::Command = std::process::Command::new(&self.executable);

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
                    cmd_fmted: format!("{} {:?}", &self.executable, &argv),
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
                    cmd_fmted: format!("{} {:?}", self.executable, argv),
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
