//! Game Server Controller (GSC).

mod gssm;

/*
 * TODO: Remove the unnecessary layer of abstraction "GameServerController":
 *       Instead, Use the container "GameServerStateMachine" directly as a top
 *       level actor!
 */
pub struct GameServerController {
    gssm: gssm::GameServerStateMachine,
    ctoken: tokio_util::sync::CancellationToken,
}

impl GameServerController {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

        cfg_client: crate::storage::GameServerConfigurationShared,
        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        tx_aggregator: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,
    ) -> Self {
        Self {
            gssm: gssm::GameServerStateMachine::init(cfg_client, rx_command, tx_aggregator, tx_activate),
            ctoken,
        }
    }

    pub async fn work(self) -> Summary {
        let ctoken = self.ctoken.child_token();
        let job = self.gssm.loop_transitions();
        let _done: Option<()> = ctoken.run_until_cancelled(job).await;
        Summary {}
    }
}

pub struct Summary;
