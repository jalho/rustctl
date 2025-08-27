pub struct RconClient {
    ctoken: tokio_util::sync::CancellationToken,
    tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

    cfg_client: crate::storage::GameServerConfigurationShared,

    /// "IGS" = "In-Game State"
    tx_agg_igs: tokio::sync::mpsc::Sender<rustctl_common::snapshot::InGameStateExposed>,
}

impl RconClient {
    pub fn new(
        ctoken: tokio_util::sync::CancellationToken,
        tx_activate: tokio::sync::mpsc::Sender<crate::actors::terminator::Activator>,

        cfg_client: crate::storage::GameServerConfigurationShared,

        tx_agg_igs: tokio::sync::mpsc::Sender<rustctl_common::snapshot::InGameStateExposed>,
    ) -> Self {
        Self {
            ctoken,
            tx_activate,

            cfg_client,

            tx_agg_igs,
        }
    }

    pub async fn work(self) -> Summary {
        let connection_string: String = self.cfg_client.get_config().await.get_rcon_connection_string();
        todo!(
            r#"connect RCON client using "{connection_string}", and then send in-game state snapshots over self.tx_agg_igs"#
        );
    }
}

pub struct Summary;
