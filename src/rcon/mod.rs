#[derive(serde::Serialize, serde::Deserialize)]
#[allow(non_snake_case)] // RCON command message's keys must be capitalized: Otherwise the game server crashes :D
struct RCONMessage<'rcon_command> {
    Message: &'rcon_command str,
    Identifier: u32,
}

pub struct RCONRelay {
    pub websocket: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
}

impl RCONRelay {
    pub fn connect(
        rx_game_server_state: std::sync::mpsc::Receiver<crate::misc::GameServerState>,
        config: &crate::args::Config,
    ) -> Result<Self, crate::error::FatalError> {
        match rx_game_server_state.recv_timeout(config.game_startup_timeout) {
            Ok(crate::misc::GameServerState::Playable) => {
                // The expected case: Game server eventually becomes playable after startup.
            }
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!(
                        "server startup completion not detected within {} minutes",
                        config.game_startup_timeout.as_secs() / 60
                    ),
                    Some(Box::new(err)),
                ));
            }
        };
        let (websocket, _) = match tungstenite::connect(format!(
            "ws://127.0.0.1:{}/{}",
            &config.rcon_port.to_string(),
            &config.rcon_password
        )) {
            Ok((websocket, http_response)) => (websocket, http_response),
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!("cannot connect WebSocket for RCON"),
                    Some(Box::new(err)),
                ));
            }
        };
        return Ok(Self { websocket });
    }

    pub fn ws_rcon_command<'rcon_command>(
        &mut self,
        rcon_command: &'rcon_command str,
    ) -> Result<(), crate::error::FatalError> {
        let command: RCONMessage = RCONMessage {
            Message: rcon_command,
            Identifier: 42,
        };
        let command_ser: String = match serde_json::to_string(&command) {
            Ok(n) => n,
            Err(err) => {
                unreachable!("RCON message payload should always be serializable as JSON");
            }
        };
        match self.websocket.send(tungstenite::Message::Text(command_ser)) {
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!("cannot send RCON command over WebSocket"),
                    Some(Box::new(err)),
                ));
            }
            _ => {}
        }
        log::debug!(
            "Sent RCON command over WebSocket: '{}' -- Waiting for response...",
            rcon_command
        );

        // TODO: Add a timeout somehow: Case we never get response with the expected identifier
        loop {
            let msg: String = match self.websocket.read() {
                Ok(n) => match n {
                    tungstenite::Message::Text(n) => n,
                    tungstenite::Message::Binary(_)
                    | tungstenite::Message::Ping(_)
                    | tungstenite::Message::Pong(_)
                    | tungstenite::Message::Close(_)
                    | tungstenite::Message::Frame(_) => {
                        return Err(crate::error::FatalError::new(format!("could not get response to RCON command: got unexpected kind of WebSocket message: {}", n), None));
                    }
                },
                Err(err) => {
                    return Err(crate::error::FatalError::new(
                        format!("cannot read RCON message over WebSocket"),
                        Some(Box::new(err)),
                    ));
                }
            };
            let msg: RCONMessage = match serde_json::from_str(&msg) {
                Ok(n) => n,
                Err(err) => {
                    return Err(crate::error::FatalError::new(
                    format!("could not get response to RCON command: could not deserialize RCON message"),
                    Some(Box::new(err)),
                ));
                }
            };
            log::debug!(
                "Got RCON message with ID {}: {}",
                msg.Identifier,
                msg.Message
            );
            if msg.Identifier == command.Identifier {
                break;
            }
        }

        return Ok(());
    }
}
