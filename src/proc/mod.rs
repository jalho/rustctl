//! Abstractions related to handling processes on the system.

static EXECUTABLE_SH: &'static str = "sh";
static EXECUTABLE_STEAMCMD: &'static str = "steamcmd";
static EXECUTABLE_RUSTDEDICATED: &'static str = "RustDedicated";

pub struct Dependency {
    executable: &'static str,
}

impl Dependency {
    pub fn init(executable: &'static str) -> Result<Self, crate::error::ErrDependencyMissing> {
        let output: std::process::Output = match std::process::Command::new(EXECUTABLE_SH)
            .arg("-c")
            .arg(format!("command -v {}", executable))
            .output()
        {
            Ok(n) => n,
            Err(_) => {
                return Err(crate::error::ErrDependencyMissing {
                    executable: EXECUTABLE_SH,
                })
            }
        };

        if !output.status.success() {
            return Err(crate::error::ErrDependencyMissing { executable });
        }

        return Ok(Self { executable });
    }
}

pub trait Exec {
    fn exec(
        &self,
        work_dir: Option<&std::path::Path>,
        argv: Vec<&str>,
        stdout_sender: std::sync::mpsc::Sender<String>,
        stderr_sender: std::sync::mpsc::Sender<String>,
        run_till_end: bool,
    ) -> Result<(), crate::error::ErrExec>;
}

impl Exec for Dependency {
    fn exec(
        &self,
        work_dir: Option<&std::path::Path>,
        argv: Vec<&str>,
        stdout_sender: std::sync::mpsc::Sender<String>,
        stderr_sender: std::sync::mpsc::Sender<String>,
        run_till_end: bool,
    ) -> Result<(), crate::error::ErrExec> {
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

        if run_till_end {
            let status = match child.wait() {
                Ok(status) => status.code(),
                Err(_) => todo!(),
            };

            let _ = stdout_thread.join();
            let _ = stderr_thread.join();

            if status != Some(0) {
                return Err(crate::error::ErrExec {
                    command: format!("{} {:?}", self.executable, argv),
                    status,
                    stderr: None, // TODO: Accumulate stderr optionally (optionally because might be too much)
                });
            }
        }

        return Ok(());
    }
}
