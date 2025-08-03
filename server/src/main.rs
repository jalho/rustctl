/*
 * Rewrite in terms of the "actor pattern" (a concurrency pattern): There should
 * be _actors_ that own their stuff (such as I/O resources), and that perform
 * work in coroutines (alias "background tasks"), and that may communicate
 * with other actors via various _channels_. The main components of the
 * program should all be actors, and the program's main functionality should be
 * implemented by arranging channels between actors.
 *
 * More terminology:
 *
 * - "downstream WebSocket client": External web clients that connect to this
 *   program to e.g. receive state updates of the managed game server and to
 *   send command messages to be passed through via "upstream RCON WebSocket
 *   client"
 *
 * - "upstream RCON WebSocket client": Command interface of the managed game
 *   server.
 */
fn main() -> std::process::ExitCode {
    let cli_args: CliArgs = <CliArgs as clap::Parser>::parse();

    let _logger_handle: log4rs::Handle = init_logging(cli_args.log_level);
    log::info!(
        "{name} {version}",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION")
    );

    /*
     * Drives (graceful) shutdown of the program upon specific signals.
     */
    let terminator: Terminator = Terminator::new();

    let config: Configuration = Configuration::resolve(cli_args.mock);

    let aggregator = StateAggregator::new(&terminator);

    let memory_querier = MemoryQuerier::new(&terminator, aggregator.get_sender_res_usage());
    let cpu_querier = CpuQuerier::new(&terminator, aggregator.get_sender_res_usage());

    let game_server_state_machine = match GameServerStateMachine::init(&config) {
        Ok(n) => n,
        Err(err) => {
            log::error!(
                "Failed to initialize game server state machine: {err_fmt}",
                err_fmt = fmt_source_tree(&err)
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    let store = Store::init(config);

    let store_shared = std::sync::Arc::new(tokio::sync::Mutex::new(store));

    let game_ctl: GameServerController = GameServerController::new(&terminator, &aggregator);

    /*
     * Stage on which downstream WebSocket clients communicate.
     */
    let stage = Stage::new(
        &terminator,
        game_ctl.get_handle(),
        aggregator.get_broadcast_handle(),
    );

    /*
     * Accepts the downstream WebSocket connections.
     */
    let web_server = WebServer::new(&terminator, &stage);

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .worker_threads(1)
        .enable_all()
        .build()
    {
        Ok(n) => n,
        Err(err) => {
            log::error!(
                "Failed to build async runtime: {err_fmt}",
                err_fmt = fmt_source_tree(&err)
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    let runtime_done = runtime.block_on(async {
        let summary = tokio::join!(
            terminator.work(),
            web_server.work(),
            stage.work(),
            game_ctl.work(game_server_state_machine, store_shared.clone()),
            aggregator.work(),
            memory_querier.work(),
            cpu_querier.work(),
        );
        summary
    });

    let (status, ..) = runtime_done;
    let exit_status: std::process::ExitCode = (&status).into();

    exit_status
}

struct MemoryQuerier {
    summary: MemoryQuerierSummary,
    cancel_read: tokio_util::sync::CancellationToken,
    slice_tx: tokio::sync::mpsc::Sender<StateUpdateSlice>,
}

struct MemoryQuerierSummary;

impl MemoryQuerier {
    pub fn new(
        terminator: &Terminator,
        slice_tx: tokio::sync::mpsc::Sender<StateUpdateSlice>,
    ) -> Self {
        let (_, cancel_read) = terminator.get_handle();

        Self {
            summary: MemoryQuerierSummary,
            cancel_read,
            slice_tx,
        }
    }

    pub async fn work(self) -> MemoryQuerierSummary {
        let query_task = async {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                let memory_usage_kibibytes = Self::read_memory_usage_kibibytes().await;

                let read_value = rustctl_common::snapshot::MemoryUsage::new(memory_usage_kibibytes);
                let read_completed_by = chrono::Utc::now();
                let queried = rustctl_common::snapshot::TimedValue {
                    read_value,
                    read_completed_by,
                };

                if self
                    .slice_tx
                    .send(StateUpdateSlice::MemoryUsageBySystemTotal(queried))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        };

        self.cancel_read.run_until_cancelled(query_task).await;
        self.summary
    }

    async fn read_memory_usage_kibibytes() -> u64 {
        let meminfo_content = tokio::fs::read_to_string("/proc/meminfo")
            .await
            .expect("Linux should have /proc/meminfo");

        let mut mem_total: u64 = 0;
        let mut mem_available: u64 = 0;

        for line in meminfo_content.lines() {
            if line.starts_with("MemTotal:") {
                mem_total = Self::parse_meminfo_line(line);
            } else if line.starts_with("MemAvailable:") {
                mem_available = Self::parse_meminfo_line(line);
            }
            if mem_total > 0 && mem_available > 0 {
                break;
            }
        }

        mem_total - mem_available
    }

    /// Parse a line from `/proc/meminfo`.
    fn parse_meminfo_line(line: &str) -> u64 {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 2 {
            unreachable!("meminfo format should be known");
        }

        let value_str = parts[1];

        value_str
            .parse::<u64>()
            .expect("meminfo format should be known")
    }
}

struct CpuQuerier {
    summary: CpuQuerierSummary,
    cancel_read: tokio_util::sync::CancellationToken,
    slice_tx: tokio::sync::mpsc::Sender<StateUpdateSlice>,
    previous_stats: Vec<CpuStats>,
}

struct CpuQuerierSummary;

/// Accurate as of:
/// ```
/// $ uname -r
/// 6.1.0-37-amd64
///
/// $ lsb_release -a
/// Description:    Debian GNU/Linux 12 (bookworm)
/// ```
#[derive(Clone, Copy)]
struct CpuStats {
    system: u64,
    user: u64,
    nice: u64,

    irq: u64,
    softirq: u64,

    steal: u64,
    guest: u64,
    guest_nice: u64,

    idle: u64,
    iowait: u64,
}

impl CpuStats {
    fn total(&self) -> u64 {
        self.guest
            + self.guest_nice
            + self.idle
            + self.iowait
            + self.irq
            + self.nice
            + self.softirq
            + self.steal
            + self.system
            + self.user
    }

    fn active(&self) -> u64 {
        self.guest
            + self.guest_nice
            // + self.idle
            // + self.iowait
            + self.irq
            + self.nice
            + self.softirq
            + self.steal
            + self.system
            + self.user
    }
}

impl CpuQuerier {
    pub fn new(
        terminator: &Terminator,
        slice_tx: tokio::sync::mpsc::Sender<StateUpdateSlice>,
    ) -> Self {
        let (_, cancel_read) = terminator.get_handle();
        Self {
            summary: CpuQuerierSummary,
            cancel_read,
            slice_tx,
            previous_stats: Vec::new(),
        }
    }

    pub async fn work(mut self) -> CpuQuerierSummary {
        let query_task = async {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            self.previous_stats = Self::read_all_cpu_stats().await;

            loop {
                interval.tick().await;
                let mut read_values = Vec::new();

                let current_stats = Self::read_all_cpu_stats().await;

                // calculate usage for each CPU
                for (cpu_index, current_cpu_stats) in current_stats.iter().enumerate() {
                    let usage_percent = if cpu_index < self.previous_stats.len() {
                        Self::calculate_cpu_usage(
                            &self.previous_stats[cpu_index],
                            current_cpu_stats,
                        )
                    } else {
                        0.0 // first reading for this CPU
                    };

                    let single_read = rustctl_common::snapshot::CpuUsage::new(usage_percent);
                    read_values.push(single_read);
                }
                self.previous_stats = current_stats;

                let read_completed_by = chrono::Utc::now();
                let queried = rustctl_common::snapshot::TimedValue {
                    read_value: read_values,
                    read_completed_by,
                };

                if self
                    .slice_tx
                    .send(StateUpdateSlice::CpuUsageBySystemTotal(queried))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        };
        self.cancel_read.run_until_cancelled(query_task).await;
        self.summary
    }

    async fn read_all_cpu_stats() -> Vec<CpuStats> {
        let stat_content = tokio::fs::read_to_string("/proc/stat")
            .await
            .expect("Linux should have /proc/stat");
        let mut cpu_stats = Vec::new();

        for line in stat_content.lines() {
            // look for lines like "cpu0", "cpu1", etc. (not the aggregate "cpu " line)
            if line.starts_with("cpu") && line.chars().nth(3).is_some_and(|c| c.is_ascii_digit()) {
                let stats = Self::parse_cpu_line(line);
                cpu_stats.push(stats);
            }
        }

        cpu_stats
    }

    /// Assuming format:
    /// ```
    /// cpu0 7856 2 1650 443198 226 0 23 0 0 0
    /// ```
    fn parse_cpu_line(line: &str) -> CpuStats {
        let mut parts: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(parts.len(), 11, "{line}");
        parts.remove(0);

        let parsed_u64: Vec<u64> = parts
            .iter()
            .map(|value| {
                let value: &str = value;
                let value: u64 = value.parse().expect("format should be known");
                value
            })
            .collect::<Vec<u64>>();

        CpuStats {
            user: parsed_u64[0],
            nice: parsed_u64[1],
            system: parsed_u64[2],
            idle: parsed_u64[3],
            iowait: parsed_u64[4],
            irq: parsed_u64[5],
            softirq: parsed_u64[6],
            steal: parsed_u64[7],
            guest: parsed_u64[8],
            guest_nice: parsed_u64[9],
        }
    }

    fn calculate_cpu_usage(prev_stats: &CpuStats, current_stats: &CpuStats) -> f64 {
        let total_diff = current_stats.total() - prev_stats.total();
        let active_diff = current_stats.active() - prev_stats.active();

        if total_diff > 0 {
            ((active_diff as f64 / total_diff as f64) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        }
    }
}

struct StateAggregator {
    summary: StateAggregatorSummary,
    cancel_read: tokio_util::sync::CancellationToken,

    aggregated_state: rustctl_common::snapshot::Snapshot,

    broadcast_tx: tokio::sync::broadcast::Sender<rustctl_common::snapshot::Snapshot>,

    slice_tx: tokio::sync::mpsc::Sender<StateUpdateSlice>,
    slice_rx: tokio::sync::mpsc::Receiver<StateUpdateSlice>,

    chan_down_state_transitions: (
        tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
        tokio::sync::mpsc::Receiver<rustctl_common::snapshot::GameServerStateExposed>,
    ),
}

struct StateAggregatorSummary;

impl StateAggregator {
    pub fn new(terminator: &Terminator) -> Self {
        let (_cancel_write, cancel_read) = terminator.get_handle();

        let (broadcast_tx, _broadcast_rx) =
            tokio::sync::broadcast::channel::<rustctl_common::snapshot::Snapshot>(1);

        let (slice_tx, slice_rx) = tokio::sync::mpsc::channel::<StateUpdateSlice>(1);

        let chan_down_state_transitions = tokio::sync::mpsc::channel(1);

        Self {
            summary: StateAggregatorSummary,
            cancel_read,
            aggregated_state: rustctl_common::snapshot::Snapshot::init(),
            broadcast_tx,
            slice_tx,
            slice_rx,
            chan_down_state_transitions,
        }
    }

    pub fn get_sender_res_usage(&self) -> tokio::sync::mpsc::Sender<StateUpdateSlice> {
        self.slice_tx.clone()
    }

    pub fn get_sender_game_server_state(
        &self,
    ) -> tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed> {
        self.chan_down_state_transitions.0.clone()
    }

    pub fn get_broadcast_handle(
        &self,
    ) -> tokio::sync::broadcast::Sender<rustctl_common::snapshot::Snapshot> {
        self.broadcast_tx.clone()
    }

    pub async fn work(mut self) -> StateAggregatorSummary {
        let mut rx_res_usage = self.slice_rx;
        let (_, mut rx_server_state) = self.chan_down_state_transitions;

        let job = async {
            loop {
                tokio::select!(
                    res_usage = rx_res_usage.recv() => {
                       match res_usage {
                            Some(StateUpdateSlice::MemoryUsageBySystemTotal(n)) => {
                                self.aggregated_state.system_memory_usage_total = n;
                            },
                            Some(StateUpdateSlice::CpuUsageBySystemTotal(n)) => {
                                self.aggregated_state.system_cpu_usage_total = n;
                            },
                            None => break,
                        }
                    },
                    server_state = rx_server_state.recv() => {
                        match server_state {
                            Some(n) => {
                                self.aggregated_state.game_server_state = n;
                            },
                            None => break,
                        }
                    },
                );
                _ = self.broadcast_tx.send(self.aggregated_state.clone());
            }
        };
        self.cancel_read.run_until_cancelled(job).await;
        self.summary
    }
}

enum StateUpdateSlice {
    MemoryUsageBySystemTotal(
        rustctl_common::snapshot::TimedValue<rustctl_common::snapshot::MemoryUsage>,
    ),

    CpuUsageBySystemTotal(
        rustctl_common::snapshot::TimedValue<Vec<rustctl_common::snapshot::CpuUsage>>,
    ),
}

struct GameServerController {
    tx: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
    rx: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
    summary: GameServerControllerSummary,
    cancel_read: tokio_util::sync::CancellationToken,
    cancel_write: tokio::sync::mpsc::Sender<()>,
    aggregator_sender: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
}
impl GameServerController {
    fn new(terminator: &Terminator, aggregator: &StateAggregator) -> Self {
        let (tx, rx) =
            tokio::sync::mpsc::channel::<rustctl_common::command::DownstreamClientMessage>(1);

        let (cancel_write, cancel_read) = terminator.get_handle();

        let aggregator_sender = aggregator.get_sender_game_server_state();

        Self {
            summary: GameServerControllerSummary,
            tx,
            rx,
            cancel_read,
            cancel_write,
            aggregator_sender,
        }
    }

    fn get_handle(
        &self,
    ) -> tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage> {
        self.tx.clone()
    }

    async fn work(
        self,
        state_machine: GameServerStateMachine,
        store: std::sync::Arc<tokio::sync::Mutex<Store>>,
    ) -> GameServerControllerSummary {
        let token: tokio_util::sync::CancellationToken = self.cancel_read.child_token();
        let job = state_machine.loop_transitions(token, self.rx, store, self.aggregator_sender);
        let done = self.cancel_read.run_until_cancelled(job).await;
        if let Some(Err(err)) = done {
            let err: NonRecoverableError = err;
            log::error!(
                "Game server controller failed: {err_fmt}",
                err_fmt = fmt_source_tree(&err)
            );
            match self.cancel_write.send(()).await {
                Ok(_) => log::debug!("Requested termination..."),
                Err(err) => log::error!(
                    "Failed to request termination: {err_fmt}",
                    err_fmt = fmt_source_tree(&err)
                ),
            }
        }
        self.summary
    }
}
struct GameServerControllerSummary;

#[allow(dead_code)] // TODO: Disallow dead code!
enum GameServerStateMachine {
    Init,
    Preparing,
    InstalledAndConfigured {
        cfg: Configuration,
    },
    Launching {
        process: tokio::process::Child,
        stdout: tokio::process::ChildStdout,
        stderr: tokio::process::ChildStderr,
    },
    RunningHealthy {
        process: tokio::process::Child,
    },
    SavingAndClosing {
        process: tokio::process::Child,
    },
    ClosedManually {
        exit_status: std::process::ExitStatus,
    },
    TerminatedUnexpectedly,
}

impl GameServerStateMachine {
    pub fn init(cfg: &Configuration) -> Result<Self, NonRecoverableError> {
        if let Some(_pid) = is_process_running(cfg.get_installer_absolute()) {
            return Err(NonRecoverableError::ConcurrentGameServerInstaller);
        }

        if let Some(_pid) = is_process_running(cfg.get_game_absolute()) {
            return Err(NonRecoverableError::ConcurrentGameServer);
        }

        Ok(Self::Init)
    }

    pub async fn loop_transitions(
        mut self,
        cancellation_token: tokio_util::sync::CancellationToken,
        mut command_rx: tokio::sync::mpsc::Receiver<
            rustctl_common::command::DownstreamClientMessage,
        >,
        store: std::sync::Arc<tokio::sync::Mutex<Store>>,
        aggregator_sender: tokio::sync::mpsc::Sender<
            rustctl_common::snapshot::GameServerStateExposed,
        >,
    ) -> Result<(), NonRecoverableError> {
        loop {
            let state_before: String = self.to_string();
            self = match self {
                Self::Init => Self::Preparing,

                /*
                 * Install or update `RustDedicated` using `steamcmd`.
                 */
                Self::Preparing => {
                    let cfg: Configuration;
                    {
                        let lock = store.lock().await;
                        cfg = lock.get_config().await;
                    }

                    let buildid_before: Option<u32> = {
                        if let Ok(contents) =
                            tokio::fs::read_to_string(cfg.get_manifest_absolute()).await
                        {
                            extract_buildid_from_buf(&contents)
                        } else {
                            None
                        }
                    };

                    let mut command = tokio::process::Command::new(cfg.get_installer_absolute());
                    command.current_dir(&cfg.root_dir_absolute);
                    command.args(cfg.get_installer_args());
                    command.stdout(std::process::Stdio::null());
                    command.stderr(std::process::Stdio::null());

                    let process: tokio::process::Child = match command.spawn() {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!(
                                "Failed to spawn game server installer ({path}): {err_fmt}",
                                path = cfg.get_installer_absolute().to_string_lossy(),
                                err_fmt = fmt_source_tree(&err),
                            );
                            return Err(NonRecoverableError::CannotSpawnGameServerInstaller);
                        }
                    };

                    let _output: std::process::Output = process.wait_with_output().await.unwrap();

                    let buildid_after: Option<u32> = {
                        if let Ok(contents) =
                            tokio::fs::read_to_string(cfg.get_manifest_absolute()).await
                        {
                            extract_buildid_from_buf(&contents)
                        } else {
                            None
                        }
                    };

                    match (buildid_before, buildid_after) {
                        (_, None) => {
                            log::error!(
                                "Installing game server failed: Could not extract buildid from game server app manifest after installation: {path}",
                                path = cfg.get_manifest_absolute().to_string_lossy()
                            );
                        }
                        (None, Some(buildid)) => {
                            log::info!("Installed game server: buildid {buildid}");
                        }
                        (Some(buildid_before), Some(buildid_after)) => {
                            if buildid_before == buildid_after {
                                log::info!(
                                    "Installation checked: Game server is up to date: buildid {buildid_after}"
                                );
                            } else {
                                log::info!(
                                    "Updated game server: From buildid {buildid_before} to {buildid_after}"
                                );
                            }
                        }
                    }

                    Self::InstalledAndConfigured { cfg }
                }

                Self::InstalledAndConfigured { cfg } => {
                    let cfg: Configuration = cfg;
                    let mut command = tokio::process::Command::new(cfg.get_game_absolute());
                    command.current_dir(&cfg.root_dir_absolute);
                    command.args(cfg.get_game_args());
                    command.stdout(std::process::Stdio::piped());
                    command.stderr(std::process::Stdio::piped());

                    let mut process: tokio::process::Child = match command.spawn() {
                        Ok(n) => n,
                        Err(err) => {
                            log::error!(
                                "Failed to spawn game server ({path}): {err_fmt}",
                                path = cfg.get_game_absolute().to_string_lossy(),
                                err_fmt = fmt_source_tree(&err),
                            );
                            return Err(NonRecoverableError::CannotSpawnGameServer);
                        }
                    };

                    let stdout: tokio::process::ChildStdout = process.stdout.take().unwrap();
                    let stderr: tokio::process::ChildStderr = process.stderr.take().unwrap();

                    Self::Launching {
                        process,
                        stdout,
                        stderr,
                    }
                }

                Self::Launching {
                    process,
                    stdout,
                    stderr,
                } => {
                    let timeout = std::time::Duration::from_secs(60 * 30); // 30 minutes
                    let mut stdout_reader = tokio::io::BufReader::new(stdout);
                    let mut stderr_reader = tokio::io::BufReader::new(stderr);

                    // channel for signaling readiness from coroutine
                    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();

                    let _read_stdout = tokio::spawn(async move {
                        let mut line = String::new();
                        let mut tx = Some(ready_tx);

                        loop {
                            line.clear();
                            match tokio::io::AsyncBufReadExt::read_line(
                                &mut stdout_reader,
                                &mut line,
                            )
                            .await
                            {
                                Ok(0) => {
                                    log::debug!("EOF reached: game server STDOUT");
                                    break;
                                }
                                Ok(_) => {
                                    let trimmed_line = line.trim_end();
                                    log::debug!(target: LOG_TARGET_GAME, "{trimmed_line}");
                                    if trimmed_line.contains("SteamServer Connected") {
                                        if let Some(sender) = tx.take() {
                                            let _ = sender.send(());
                                        }
                                    }
                                }
                                Err(err) => {
                                    log::error!(
                                        "Failed to read line from STDOUT: {err_fmt}",
                                        err_fmt = fmt_source_tree(&err)
                                    );
                                    break;
                                }
                            }
                        }
                    });

                    let _read_stderr = tokio::spawn(async move {
                        let mut line = String::new();

                        loop {
                            line.clear();
                            match tokio::io::AsyncBufReadExt::read_line(
                                &mut stderr_reader,
                                &mut line,
                            )
                            .await
                            {
                                Ok(0) => {
                                    log::debug!("EOF reached: game server STDERR");
                                    break;
                                }
                                Ok(_) => {
                                    let trimmed_line = line.trim_end();
                                    log::debug!(target: LOG_TARGET_GAME, "{trimmed_line}");
                                }
                                Err(err) => {
                                    log::error!(
                                        "Failed to read line from STDERR: {err_fmt}",
                                        err_fmt = fmt_source_tree(&err)
                                    );
                                    break;
                                }
                            }
                        }
                    });

                    let wait_readiness = async {
                        if let Err(err) = ready_rx.await {
                            let err: tokio::sync::oneshot::error::RecvError = err;
                            if !cancellation_token.is_cancelled() {
                                /*
                                 * The Err variant is expected when the channel gets teared
                                 * down, which is expected to happen when the program is
                                 * about to terminate, as indicated by the cancellation
                                 * token.
                                 *
                                 * If the Err variant is reached in any other scenario, then
                                 * that's a bug that should be investigated!
                                 */
                                todo!(
                                    "readiness channel receive failed while not cancelled: {err_fmt}",
                                    err_fmt = fmt_source_tree(&err)
                                );
                            }
                        }
                    };

                    match tokio::time::timeout(timeout, wait_readiness).await {
                        Ok(_) => Self::RunningHealthy { process },
                        Err(err) => {
                            log::error!(
                                "Game server did not indicate its readiness within timeout of {timeout_secs} seconds: {err_fmt}",
                                timeout_secs = timeout.as_secs(),
                                err_fmt = fmt_source_tree(&err)
                            );
                            return Err(NonRecoverableError::GameServerStartupTimeout);
                        }
                    }
                }

                Self::RunningHealthy { mut process } => {
                    let event: GameCtlEvent = tokio::select! {
                        msg = command_rx.recv() => {
                            match msg {
                                Some(message) => GameCtlEvent::MessageReceived { message },
                                None => GameCtlEvent::MessageChannelClosed,
                            }
                        }
                        output = process.wait() => {
                            let exit_status: std::process::ExitStatus = output.unwrap();
                            GameCtlEvent::GameProcessTerminated { exit_status }
                        }
                    };

                    match event {
                        GameCtlEvent::MessageReceived { message } => {
                            let command: rustctl_common::command::DownstreamClientMessage = message;
                            match command {
                                rustctl_common::command::DownstreamClientMessage::ServerSaveAndClose => {
                                     let signal = nix::sys::signal::Signal::SIGINT;
                                     let pid = send_signal(&process, signal).await;
                                     log::info!("Sent signal to game server process: {signal}: PID {pid}");
                                     Self::SavingAndClosing { process }
                                }
                                _ => {
                                    log::error!(
                                        "Ignoring unexpected command: {command:?} for current state"
                                    );
                                    Self::RunningHealthy { process }
                                }
                            }
                        }
                        GameCtlEvent::MessageChannelClosed => todo!(),
                        GameCtlEvent::GameProcessTerminated { exit_status } => {
                            let _exit_status: std::process::ExitStatus = exit_status;
                            Self::TerminatedUnexpectedly
                        }
                    }
                }

                Self::SavingAndClosing { mut process } => {
                    /*
                     * TODO: Wait until some timeout only! And get the actual
                     *       game server root dir & expected save file name from
                     *       some existing structure...
                     *
                     *       Example (relative to the game server executable):
                     *
                     *       $ ls -lt server/instance0/
                     *       total 2772
                     *       -rw-r--r-- 1 jka jka   68660 Aug  3 17:58 proceduralmap.1000.1337.269.sav
                     *       -rw-r--r-- 1 jka jka   68975 Aug  3 17:48 proceduralmap.1000.1337.269.sav.1
                     */
                    let saved: std::fs::Metadata = wait_file(
                        std::path::Path::new("game server root dir"),
                        std::path::Path::new("savefile.txt"),
                    )
                    .await;
                    log::info!("game server state saved: {saved:?}");

                    let exit_status = match process.wait().await {
                        Ok(n) => {
                            log::info!("game server process exited with status {n}");
                            n
                        }
                        Err(err) => {
                            todo!("waiting for game server process to terminate failed: {err}");
                        }
                    };

                    Self::ClosedManually { exit_status }
                }

                Self::ClosedManually { .. } => {
                    let msg = command_rx.recv().await;
                    if let Some(command) = msg {
                        let command: rustctl_common::command::DownstreamClientMessage = command;
                        match command {
                            rustctl_common::command::DownstreamClientMessage::ServerInstallOrUpdateAndStart => {
                                Self::Preparing
                            }
                            _ => {
                                log::error!(
                                    "Ignoring unexpected command: {command:?} for current state: {self}"
                                );
                                self
                            }
                        }
                    } else {
                        self
                    }
                }

                Self::TerminatedUnexpectedly => Self::Preparing,
            };
            let state_after: String = self.to_string();
            log::info!("Transitioned: {state_before} -> {state_after}");
            if let Err(_err) = aggregator_sender.send((&self).into()).await {
                /*
                 * Channel being closed indicates that the program is doing
                 * a shutdown.
                 *
                 * TODO: Verify by checking the cancellation token?
                 */
                return Ok(());
            }
        }
    }
}
impl std::fmt::Display for GameServerStateMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameServerStateMachine::Init => write!(f, "Init"),
            GameServerStateMachine::Preparing => write!(f, "Preparing"),
            GameServerStateMachine::InstalledAndConfigured { .. } => {
                write!(f, "InstalledAndConfigured")
            }
            GameServerStateMachine::Launching { .. } => write!(f, "Launching"),
            GameServerStateMachine::RunningHealthy { .. } => write!(f, "RunningHealthy"),
            GameServerStateMachine::SavingAndClosing { .. } => write!(f, "SavingAndClosing"),
            GameServerStateMachine::ClosedManually { .. } => write!(f, "ClosedManually"),
            GameServerStateMachine::TerminatedUnexpectedly => write!(f, "TerminatedUnexpectedly"),
        }
    }
}

impl From<&GameServerStateMachine> for rustctl_common::snapshot::GameServerStateExposed {
    fn from(value: &GameServerStateMachine) -> Self {
        match value {
            GameServerStateMachine::Init => rustctl_common::snapshot::GameServerStateExposed::Init,
            GameServerStateMachine::Preparing => {
                rustctl_common::snapshot::GameServerStateExposed::Preparing
            }
            GameServerStateMachine::InstalledAndConfigured { .. } => {
                rustctl_common::snapshot::GameServerStateExposed::InstalledAndConfigured
            }
            GameServerStateMachine::Launching { .. } => {
                rustctl_common::snapshot::GameServerStateExposed::Launching
            }
            GameServerStateMachine::RunningHealthy { .. } => {
                rustctl_common::snapshot::GameServerStateExposed::RunningHealthy
            }
            GameServerStateMachine::SavingAndClosing { .. } => {
                rustctl_common::snapshot::GameServerStateExposed::SavingAndClosing
            }
            GameServerStateMachine::ClosedManually { .. } => {
                rustctl_common::snapshot::GameServerStateExposed::ClosedManually
            }
            GameServerStateMachine::TerminatedUnexpectedly => {
                rustctl_common::snapshot::GameServerStateExposed::TerminatedUnexpectedly
            }
        }
    }
}

#[allow(dead_code)] // TODO: Disallow dead code!
#[derive(Debug, Clone)]
struct Configuration {
    root_dir_absolute: std::path::PathBuf,
    installer_relative: std::path::PathBuf,
    game_relative: std::path::PathBuf,
    manifest_relative: std::path::PathBuf,

    game_world_size: u16,
    game_world_seed: u32,

    rcon_port: u16,
    rcon_password: String,
}
impl Configuration {
    pub fn resolve(mock: bool) -> Self {
        if !mock {
            todo!("only --mock mode is implemented for now");
        } else {
            let game_world_size = 1000;
            let game_world_seed = 1337;
            let rcon_port = 28016;
            let rcon_password = uuid::Uuid::new_v4().to_string();

            let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let crate_root_abs = crate_root.canonicalize().unwrap();
            let workspace_root_abs = crate_root_abs
                .parent()
                .expect("crate root should have parent")
                .to_path_buf();

            Self {
                root_dir_absolute: workspace_root_abs,
                installer_relative: std::path::Path::new(
                    "target/x86_64-unknown-linux-musl/debug/steamcmd",
                )
                .to_path_buf(),
                game_relative: std::path::Path::new(
                    "target/x86_64-unknown-linux-musl/debug/RustDedicated",
                )
                .to_path_buf(),
                manifest_relative: std::path::Path::new("mocks/dummy_manifest.acf").to_path_buf(),
                game_world_size,
                game_world_seed,
                rcon_port,
                rcon_password,
            }
        }
    }

    pub fn get_installer_absolute(&self) -> std::path::PathBuf {
        let mut path = self.root_dir_absolute.clone();
        path.push(&self.installer_relative);
        path
    }

    pub fn get_game_absolute(&self) -> std::path::PathBuf {
        let mut path = self.root_dir_absolute.clone();
        path.push(&self.game_relative);
        path
    }

    pub fn get_manifest_absolute(&self) -> std::path::PathBuf {
        let mut path = self.root_dir_absolute.clone();
        path.push(&self.manifest_relative);
        path
    }

    pub fn get_installer_args(&self) -> Vec<String> {
        vec![
            "+login".into(),
            "anonymous".into(),
            /*
             * WONTFIX: "force_install_dir" doesn't really "force" anything:
             *          Instead, SteamCMD seems to just create a new directory
             *          tree in "~/.local/share/Steam/" if it cannot access
             *          the given "force_install_dir". Therefore, we should
             *          add some checks to actually know where the installation
             *          ends up at. However, this is low priority as long as the
             *          specified directory is owned by the current user and so
             *          we can assume the command does what it's told to do.
             *
             *          Side note (opinionated): For SteamCMD, a more correct
             *          API would be to exit with failure status if a location
             *          that was requested "forced" cannot be used, and to NOT
             *          try to silently use some other location.
             *
             *          Behavior observed in `apt` packaged version:
             *          - Package: steamcmd:i386
             *          - Version: 0~20180105-5 (latest as of July 2025)
             *          - Section: non-free/games
             *          - Maintainer: Debian Games Team
             */
            "+force_install_dir".into(),
            self.root_dir_absolute.to_string_lossy().to_string(),
            "+app_update".into(),
            "258550".into(),
            "validate".into(),
            "+quit".into(),
        ]
    }

    pub fn get_game_args(&self) -> Vec<String> {
        vec![
            "-batchmode".into(),
            "+server.identity".into(),
            "instance0".into(),
            "+rcon.port".into(),
            self.rcon_port.to_string(),
            "+rcon.web".into(),
            "1".into(),
            "+rcon.password".into(),
            self.rcon_password.clone(),
        ]
    }
}

struct Terminator {
    summary: TerminatorSummary,
    cancellation_token: tokio_util::sync::CancellationToken,
    cancellation_channel: (
        tokio::sync::mpsc::Sender<()>,
        tokio::sync::mpsc::Receiver<()>,
    ),
}
impl Terminator {
    pub fn new() -> Self {
        Self {
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            summary: TerminatorSummary(None),
            cancellation_channel: tokio::sync::mpsc::channel(1),
        }
    }

    pub async fn work(mut self) -> TerminatorSummary {
        let coroutine = tokio::spawn(async move {
            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
            let exit_code: Option<std::process::ExitCode> = tokio::select! {
                _ = sigint.recv() => {
                    log::info!("SIGINT");
                    None
                },
                _ = sigterm.recv() => {
                    log::info!("SIGTERM");
                    None
                },
                _ = self.cancellation_channel.1.recv() => {
                    log::info!("Cancellation requested");
                    Some(std::process::ExitCode::FAILURE)
                },
            };
            self.cancellation_token.cancel();
            exit_code
        });

        let done = coroutine.await;
        if let Ok(Some(exit_code)) = done {
            self.summary = TerminatorSummary(Some(exit_code));
        }

        self.summary
    }

    pub fn get_handle(
        &self,
    ) -> (
        tokio::sync::mpsc::Sender<()>,
        tokio_util::sync::CancellationToken,
    ) {
        (
            self.cancellation_channel.0.clone(),
            self.cancellation_token.child_token(),
        )
    }
}

struct TerminatorSummary(Option<std::process::ExitCode>);
impl From<&TerminatorSummary> for std::process::ExitCode {
    fn from(value: &TerminatorSummary) -> Self {
        match value.0 {
            Some(exit_code) => exit_code,
            None => std::process::ExitCode::SUCCESS,
        }
    }
}

fn init_logging(level: log::LevelFilter) -> log4rs::Handle {
    const APPENDER_NAME_CORE: &str = "core";
    const APPENDER_NAME_GAME: &str = "game_server";

    let appender_core: log4rs::append::console::ConsoleAppender =
        log4rs::append::console::ConsoleAppender::builder()
            .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                "{h({d(%Y-%m-%d %H:%M:%S)(utc)} [rustctl] {m})} [{f}:{L}]\n",
            )))
            .build();

    let appender_game: log4rs::append::console::ConsoleAppender =
        log4rs::append::console::ConsoleAppender::builder()
            .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                "{h({d(%Y-%m-%d %H:%M:%S)(utc)} [{t}] {m})}\n",
            )))
            .build();

    let appender_cfg_core: log4rs::config::Appender =
        log4rs::config::Appender::builder().build(APPENDER_NAME_CORE, Box::new(appender_core));

    let appender_cfg_game: log4rs::config::Appender =
        log4rs::config::Appender::builder().build(APPENDER_NAME_GAME, Box::new(appender_game));

    let config = log4rs::Config::builder()
        .appender(appender_cfg_core)
        .appender(appender_cfg_game)
        .logger(
            log4rs::config::Logger::builder()
                .appender(APPENDER_NAME_GAME)
                .additive(false) // log only for the specific target, i.e. don't propagate duplicate log
                .build(LOG_TARGET_GAME, level),
        )
        .build(
            log4rs::config::Root::builder()
                .appender(APPENDER_NAME_CORE)
                .build(level),
        )
        .unwrap();

    log4rs::init_config(config).unwrap()
}

#[derive(clap::Parser, Debug)]
#[command(version)]
pub struct CliArgs {
    #[arg(short, long, default_value_t = log::LevelFilter::Debug)]
    pub log_level: log::LevelFilter,

    #[arg(long, default_value_t = false)]
    pub mock: bool,
}

#[derive(Clone)]
struct WebServerState {
    stage: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
    broadcast_handle: tokio::sync::broadcast::Sender<rustctl_common::snapshot::Snapshot>,
    clients_connected:
        std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<uuid::Uuid, DownstreamClient>>>,
}

impl WebServerState {
    pub fn new(
        stage: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
        broadcast_handle: tokio::sync::broadcast::Sender<rustctl_common::snapshot::Snapshot>,
    ) -> Self {
        Self {
            stage,
            broadcast_handle,
            clients_connected: std::sync::Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    pub fn get_stage_handle(
        &self,
    ) -> tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage> {
        self.stage.clone()
    }

    pub fn get_broadcast_handle(
        &self,
    ) -> tokio::sync::broadcast::Sender<rustctl_common::snapshot::Snapshot> {
        self.broadcast_handle.clone()
    }

    pub async fn register_client(&mut self, client: DownstreamClient) -> (uuid::Uuid, usize) {
        let id = uuid::Uuid::new_v4();
        let connected_total: usize;

        {
            let mut lock = self.clients_connected.lock().await;
            lock.insert(id, client);
            connected_total = lock.len();
        }

        (id, connected_total)
    }

    pub async fn unregister_client(&mut self, id: &uuid::Uuid) -> usize {
        let connected_remaining: usize;

        {
            let mut lock = self.clients_connected.lock().await;
            lock.remove(id);
            connected_remaining = lock.len();
        }

        connected_remaining
    }
}

struct WebServer {
    summary: WebServerSummary,
    cancel_read: tokio_util::sync::CancellationToken,
    router: axum::Router,
}
impl WebServer {
    pub fn new(terminator: &Terminator, stage: &Stage) -> Self {
        let state = WebServerState::new(stage.get_handle(), stage.get_broadcast_handle());

        let router: axum::Router = axum::Router::new()
            .route(
                rustctl_common::web_app::WEBSOCKET_CONNECT_URL_PATH,
                axum::routing::get(websocket_handler),
            )
            .with_state(state);

        Self {
            summary: WebServerSummary {},
            cancel_read: terminator.get_handle().1,
            router,
        }
    }

    pub async fn work(self) -> WebServerSummary {
        let tcp_listener = match tokio::net::TcpListener::bind("127.0.0.1:8080").await {
            Ok(n) => n,
            Err(err) => {
                log::error!(
                    "Failed to bind TCP listener: {err_fmt}",
                    err_fmt = fmt_source_tree(&err)
                );
                return self.summary;
            }
        };

        let service = self
            .router
            .into_make_service_with_connect_info::<std::net::SocketAddr>();

        let serve = axum::serve(tcp_listener, service);

        if let Some(Err(err)) = self
            .cancel_read
            .run_until_cancelled(async move { serve.await })
            .await
        {
            /*
             * From docs (axum v0.8.4):
             *   fn axum::serve "will never actually complete or return an error"
             */
            unreachable!("{err}")
        }
        self.summary
    }
}
struct WebServerSummary;

async fn websocket_handler(
    ws: axum::extract::WebSocketUpgrade,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    axum::extract::State(state): axum::extract::State<WebServerState>,
) -> axum::response::Response {
    ws.on_upgrade(async move |socket| {
        let socket: axum::extract::ws::WebSocket = socket;
        let addr: std::net::SocketAddr = addr;
        let mut state: WebServerState = state;

        let client = DownstreamClient::new();
        let (client_id, connected_total) = state.register_client(client).await;
        log::info!("Downstream client connected: {addr} -- {connected_total} connected clients in total");

        let (tx, rx) = futures_util::StreamExt::split(socket);
        let mut sender = DownstreamClientSender::new(tx, state.get_broadcast_handle());
        let mut receiver = DownstreamClientReceiver::new(rx);

        let _done: () = tokio::select!(
            done = sender.work() => done,
            done = receiver.work(state.get_stage_handle()) => done,
        );

        let connected_remaining = state.unregister_client(&client_id).await;
        log::info!(
            "Downstream client disconnected: {addr} -- {connected_remaining} connected clients remain"
        );
    })
}

#[derive(Clone)]
struct DownstreamClient {}

impl DownstreamClient {
    pub fn new() -> Self {
        Self {}
    }
}

struct DownstreamClientSender {
    tx: futures_util::stream::SplitSink<axum::extract::ws::WebSocket, axum::extract::ws::Message>,
    state_rx: tokio::sync::broadcast::Receiver<rustctl_common::snapshot::Snapshot>,
}

impl DownstreamClientSender {
    pub fn new(
        tx: futures_util::stream::SplitSink<
            axum::extract::ws::WebSocket,
            axum::extract::ws::Message,
        >,
        broadcast_handle: tokio::sync::broadcast::Sender<rustctl_common::snapshot::Snapshot>,
    ) -> Self {
        Self {
            tx,
            state_rx: broadcast_handle.subscribe(),
        }
    }

    pub async fn work(&mut self) {
        'send_messages: loop {
            let snapshot = match self.state_rx.recv().await {
                Ok(snapshot) => snapshot,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    log::debug!("State broadcast channel closed");
                    break 'send_messages;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    log::warn!("Client lagged behind, skipped {skipped} state updates");
                    continue;
                }
            };

            let serialized: String = match serde_json::to_string(&snapshot) {
                Ok(s) => s,
                Err(err) => {
                    log::error!("Failed to serialize snapshot: {err}");
                    continue;
                }
            };

            let send = futures_util::SinkExt::send(&mut self.tx, serialized.into());
            if let Err(err) = send.await {
                let err: axum::Error = err;
                log::error!(
                    "Failed to send message to downstream client: {err_fmt}",
                    err_fmt = fmt_source_tree(&err)
                );
                break 'send_messages;
            }
        }
    }
}

struct DownstreamClientReceiver {
    rx: futures_util::stream::SplitStream<axum::extract::ws::WebSocket>,
}

impl DownstreamClientReceiver {
    pub fn new(rx: futures_util::stream::SplitStream<axum::extract::ws::WebSocket>) -> Self {
        Self { rx }
    }

    pub async fn work(
        &mut self,
        stage: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
    ) {
        'recv_messages: loop {
            let next = futures_util::StreamExt::next(&mut self.rx);
            let msg: axum::extract::ws::Message = match next.await {
                Some(Ok(n)) => n,
                Some(Err(err)) => {
                    /*
                     * Client closing the connection non-gracefully is not the
                     * happy path, but there might also not be anything we can
                     * do about it (e.g. in the case of the client's networking
                     * device just exploding or something), therefore logging
                     * as warning.
                     */
                    log::warn!(
                        "Client likely closed non-gracefully: Failed to receive message from downstream client: {err_fmt}",
                        err_fmt = fmt_source_tree(&err)
                    );
                    break 'recv_messages;
                }
                None => {
                    break 'recv_messages;
                }
            };

            /*
             * TODO: Implement graceful disconnect: Unregister the client when
             *       graceful close message is received!
             */
            let msg: rustctl_common::command::DownstreamClientMessage = match (&msg).try_into() {
                Ok(n) => n,
                Err(err) => {
                    /*
                     * Client misbehavior indicates a bug in the client, in
                     * which case we should drop it.
                     */
                    log::error!(
                        "Received invalid message from a downstream client: {err_fmt} -- Stopping handling!",
                        err_fmt = fmt_source_tree(&err)
                    );
                    break 'recv_messages;
                }
            };

            if let Err(err) = stage.send(msg).await {
                log::error!(
                    "Failed to send downstream client message to stage: {err_fmt}",
                    err_fmt = fmt_source_tree(&err)
                );
                /*
                 * Not being able to send a message to stage may indicate a
                 * non-recoverable error case! (In case e.g. out of memory or
                 * something...)
                 */
                todo!("request termination of the program");
            }
        }
    }
}

struct Stage {
    summary: StageSummary,
    cancel_read: tokio_util::sync::CancellationToken,
    channel: (
        tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
        tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
    ),
    game_ctl: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
    broadcast_handle: tokio::sync::broadcast::Sender<rustctl_common::snapshot::Snapshot>,
}

impl Stage {
    fn new(
        terminator: &Terminator,
        game_ctl: tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
        broadcast_handle: tokio::sync::broadcast::Sender<rustctl_common::snapshot::Snapshot>,
    ) -> Self {
        Self {
            channel: tokio::sync::mpsc::channel(1),
            cancel_read: terminator.get_handle().1,
            summary: StageSummary { messages_total: 0 },
            game_ctl,
            broadcast_handle,
        }
    }

    fn get_handle(
        &self,
    ) -> tokio::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage> {
        let (tx, _rx) = &self.channel;
        tx.clone()
    }

    fn get_broadcast_handle(
        &self,
    ) -> tokio::sync::broadcast::Sender<rustctl_common::snapshot::Snapshot> {
        self.broadcast_handle.clone()
    }

    async fn work(mut self) -> StageSummary {
        let (_tx, mut rx) = self.channel;
        let coroutine = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Some(no_overflow) = self.summary.messages_total.checked_add(1) {
                    self.summary.messages_total = no_overflow;
                }
                let msg: rustctl_common::command::DownstreamClientMessage = msg;
                if let Err(err) = self.game_ctl.send(msg).await {
                    log::error!(
                        "Failed to send downstream client message from stage to game server controller: {err_fmt}",
                        err_fmt = fmt_source_tree(&err)
                    );
                }
            }
        });
        _ = self.cancel_read.run_until_cancelled(coroutine).await;
        self.summary
    }
}

struct StageSummary {
    messages_total: u128,
}

struct Store {
    in_mem: Configuration,
}
impl Store {
    pub fn init(initial_config: Configuration) -> Self {
        Self {
            in_mem: initial_config,
        }
    }

    pub async fn get_config(&self) -> Configuration {
        self.in_mem.clone()
    }
}

fn extract_buildid_from_buf(buf: &str) -> Option<u32> {
    let vdf: keyvalues_parser::Vdf = match keyvalues_parser::Vdf::parse(buf) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let root: &keyvalues_parser::Obj = vdf.value.get_obj()?;

    let buildid_str: &str = match root.get("buildid") {
        Some(values) => {
            if values.len() != 1 {
                return None;
            }
            values[0].get_str()?
        }
        None => return None,
    };

    buildid_str.parse::<u32>().ok()
}

#[derive(Debug)]
enum NonRecoverableError {
    /// Game server installer is running when it is not expected to be.
    ConcurrentGameServerInstaller,

    /// Game server is running when it is not expected to be.
    ConcurrentGameServer,

    /// Cannot spawn `steamcmd`.
    CannotSpawnGameServerInstaller,

    /// Cannot spawn `RustDedicated`.
    CannotSpawnGameServer,

    /// Launched game server did not pass health check within timeout.
    GameServerStartupTimeout,
}

impl std::error::Error for NonRecoverableError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NonRecoverableError::ConcurrentGameServerInstaller => None,
            NonRecoverableError::ConcurrentGameServer => None,
            NonRecoverableError::CannotSpawnGameServerInstaller => None,
            NonRecoverableError::CannotSpawnGameServer => None,
            NonRecoverableError::GameServerStartupTimeout => None,
        }
    }
}

impl std::fmt::Display for NonRecoverableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NonRecoverableError::ConcurrentGameServerInstaller => {
                write!(f, "game server installer running already")
            }
            NonRecoverableError::CannotSpawnGameServerInstaller => {
                write!(f, "cannot spawn process for game server installer")
            }
            NonRecoverableError::ConcurrentGameServer => {
                write!(f, "game server running already")
            }
            NonRecoverableError::CannotSpawnGameServer => {
                write!(f, "cannot spawn process for game server")
            }
            NonRecoverableError::GameServerStartupTimeout => {
                write!(f, "game server startup timeout")
            }
        }
    }
}

const LOG_TARGET_GAME: &str = "game";

enum GameCtlEvent {
    MessageReceived {
        message: rustctl_common::command::DownstreamClientMessage,
    },

    MessageChannelClosed,

    GameProcessTerminated {
        exit_status: std::process::ExitStatus,
    },
}

/// Like `pgrep`: Check if there's a program with given name running. Returns
/// the running process's ID (PID) if so.
fn is_process_running(executable: std::path::PathBuf) -> Option<u32> {
    let name = match executable.file_name() {
        Some(n) => n,
        None => {
            return None;
        }
    };

    let proc_dir = match std::fs::read_dir("/proc") {
        Ok(dir) => dir,
        Err(_) => return None,
    };

    let seekable: &str = name.to_str()?;

    for entry in proc_dir.flatten() {
        let item: std::ffi::OsString = entry.file_name();
        let item: &str = item.to_str()?;

        if item.chars().all(|c| c.is_ascii_digit()) {
            let pid_path = entry.path();

            // comm file: contains the process name
            let comm = pid_path.join("comm");
            if let Ok(buf) = std::fs::read_to_string(&comm) {
                let process_name: &str = buf.trim();
                if process_name == seekable {
                    if let Ok(pid) = item.parse::<u32>() {
                        return Some(pid);
                    }
                }
            }
        }
    }

    None
}

fn fmt_source_tree<E>(error: &E) -> String
where
    E: std::error::Error,
{
    let mut concatenated: String = String::new();

    concatenated.push_str(&format!("{error}"));

    let mut source: Option<_> = error.source();
    while let Some(src) = source {
        concatenated.push_str(&format!(": {src}"));
        source = src.source();
    }

    concatenated
}

async fn send_signal(
    child: &tokio::process::Child,
    signal: nix::sys::signal::Signal,
) -> nix::unistd::Pid {
    let pid: u32 = child.id().expect("process should have PID");
    let pid: i32 = pid.try_into().expect("PID u32 should fit into i32: {pid}");
    let pid: nix::unistd::Pid = nix::unistd::Pid::from_raw(pid);
    _ = nix::sys::signal::kill(pid, signal).expect("sending a signal should succeed");
    pid
}

async fn wait_file(dir: &std::path::Path, file_name: &std::path::Path) -> std::fs::Metadata {
    let file_path = dir.join(file_name);
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        match tokio::fs::metadata(&file_path).await {
            Ok(metadata) => return metadata.into(),
            Err(_) => continue,
        }
    }
}
