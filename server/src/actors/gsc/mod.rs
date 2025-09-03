//! Game Server Controller (GSC).

pub mod gssm;

pub struct GameServerController {
    gssm: gssm::GameServerStateMachine,
    ctoken: tokio_util::sync::CancellationToken,
}

impl GameServerController {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

        cfg_client: crate::storage::ConfigurationClient,

        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,

        tx_agg_gss: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,

        tx_rconready: tokio::sync::mpsc::Sender<crate::actors::gsc::gssm::ReadyForRcon>,
    ) -> Self {
        Self {
            gssm: gssm::GameServerStateMachine::init(tx_activate, cfg_client, rx_command, tx_agg_gss, tx_rconready),
            ctoken,
        }
    }

    /*
     * TODO: In graceful shutdown sequence:
     *
     *       1. Issue "save game state" command to the game server via RCON
     *       2. Give the game some time to do the save.
     *       3. Signal the whole spawned child process group to terminate.
     *
     *       In current impl, the spawned child process group is left running on
     *       SIGINT. Termination of the child group works as intended when done
     *       via commanding downstream WebSocket clients though, because there
     *       the group ID is used accordingly!
     *
     * TODO: At startup (of the game server state machine?), it should be
     *       asserted that none of the spawnable workloads are not already
     *       running: `steamcmd`, `RustDedicated` etc. That should act as a
     *       sanity check: This program is intended to manage a single instance
     *       of the game at a time, and therefore concurrent instances should be
     *       considered undefined behavior!
     */
    pub async fn work(self) -> Summary {
        let ctoken = self.ctoken.child_token();
        let job = self.gssm.loop_transitions();
        let _done: Option<()> = ctoken.run_until_cancelled(job).await;
        Summary {}
    }
}

pub struct Summary;
