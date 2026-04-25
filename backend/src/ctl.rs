/// Command for _restarting web server_.
pub struct CommandFromController;

#[derive(Debug)]
pub enum CommandFromWebClient {}

pub async fn handle_commands_from_web_clients(
    mut rx_cmd_from_web_client: tokio::sync::mpsc::Receiver<CommandFromWebClient>,
    _tx_cmd_from_controller: tokio::sync::mpsc::Sender<CommandFromController>,
) {
    loop {
        if let Some(n) = rx_cmd_from_web_client.recv().await {
            dbg!(n);
        }
    }
}
