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

        cfg_client: crate::storage::GameServerConfigurationShared,

        rx_command: tokio::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,

        tx_agg_gss: tokio::sync::mpsc::Sender<rustctl_common::snapshot::GameServerStateExposed>,

        tx_rconready: tokio::sync::mpsc::Sender<crate::actors::gsc::gssm::ReadyForRcon>,
    ) -> Self {
        Self {
            gssm: gssm::GameServerStateMachine::init(tx_activate, cfg_client, rx_command, tx_agg_gss, tx_rconready),
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
