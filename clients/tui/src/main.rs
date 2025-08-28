fn main() -> std::process::ExitCode {
    let c0: tokio_util::sync::CancellationToken = tokio_util::sync::CancellationToken::new();
    let c1: tokio_util::sync::CancellationToken = c0.child_token();

    let (tx_updates, rx_updates) = std::sync::mpsc::channel::<rustctl_common::snapshot::Snapshot>();
    let (tx_commands, rx_commands) = std::sync::mpsc::channel::<rustctl_common::command::DownstreamClientMessage>();

    let th_tui = std::thread::spawn(|| tui::work(rx_updates, tx_commands, c0));
    let th_connection = std::thread::spawn(|| connection::work(tx_updates, rx_commands, c1));

    let _done_tui: () = th_tui.join().unwrap();
    let _done_connection: () = th_connection.join().unwrap();

    ratatui::restore();
    std::process::ExitCode::SUCCESS
}

mod connection {
    use futures_util::{SinkExt, StreamExt};
    use rustctl_common::web_app::WEBSOCKET_CONNECT_URL_PATH;

    pub fn work(
        tx_updates: std::sync::mpsc::Sender<rustctl_common::snapshot::Snapshot>,
        rx_commands: std::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
        cancel: tokio_util::sync::CancellationToken,
    ) {
        let rt: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();

        let job = cancel.run_until_cancelled(connect(tx_updates, rx_commands));

        let _coroutine_done = rt.block_on(job);
    }

    async fn connect(
        tx_updates: std::sync::mpsc::Sender<rustctl_common::snapshot::Snapshot>,
        rx_commands: std::sync::mpsc::Receiver<rustctl_common::command::DownstreamClientMessage>,
    ) {
        let (stream, _response) =
            tokio_tungstenite::connect_async(format!("ws://127.0.0.1:8080{WEBSOCKET_CONNECT_URL_PATH}"))
                .await
                .unwrap();

        let (mut write, mut read) = stream.split();

        let coroutine_pass_commands = tokio::spawn(async move {
            'pass_commands: loop {
                if let Ok(command) = rx_commands.try_recv() {
                    let serialized = serde_json::to_string(&command).unwrap();
                    let message = tokio_tungstenite::tungstenite::Message::Text(serialized.into());
                    if write.send(message).await.is_err() {
                        break 'pass_commands;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });

        'recv_messages: while let Some(Ok(msg)) = read.next().await {
            let msg: tokio_tungstenite::tungstenite::Message = msg;
            let utf8: String = match msg {
                tokio_tungstenite::tungstenite::Message::Text(utf8_bytes) => utf8_bytes.to_string(),
                _ => {
                    break 'recv_messages;
                }
            };

            let deserialized: rustctl_common::snapshot::Snapshot = serde_json::from_str(&utf8).unwrap();

            if tx_updates.send(deserialized).is_err() {
                break 'recv_messages;
            }
        }

        coroutine_pass_commands.abort();
    }
}

mod tui {
    use ratatui::prelude::Widget;

    pub fn work(
        rx_updates: std::sync::mpsc::Receiver<rustctl_common::snapshot::Snapshot>,
        tx_commands: std::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
        cancel: tokio_util::sync::CancellationToken,
    ) {
        let mut terminal: ratatui::Terminal<_> = ratatui::init();
        Ctl::new(rx_updates, tx_commands, cancel).run(&mut terminal).unwrap();
    }

    pub struct Ctl {
        should_terminate: tokio_util::sync::CancellationToken,
        rx_updates: std::sync::mpsc::Receiver<rustctl_common::snapshot::Snapshot>,
        tx_commands: std::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
        latest_snapshot: Option<rustctl_common::snapshot::Snapshot>,
    }

    impl Ctl {
        pub fn new(
            rx_updates: std::sync::mpsc::Receiver<rustctl_common::snapshot::Snapshot>,
            tx_commands: std::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
            cancel: tokio_util::sync::CancellationToken,
        ) -> Self {
            Self {
                should_terminate: cancel,
                rx_updates,
                tx_commands,
                latest_snapshot: None,
            }
        }

        pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
            while !self.should_terminate.is_cancelled() {
                // Update with latest snapshot if available
                while let Ok(snapshot) = self.rx_updates.try_recv() {
                    self.latest_snapshot = Some(snapshot);
                }

                terminal.draw(|frame| self.draw(frame))?;

                if crossterm::event::poll(std::time::Duration::from_millis(100))? {
                    let event = crossterm::event::read()?;
                    if let crossterm::event::Event::Key(key_event) = event {
                        self.handle_key_event(key_event);
                    }
                }
            }
            Ok(())
        }

        fn draw(&self, frame: &mut ratatui::Frame) {
            frame.render_widget(self, frame.area());
        }

        fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) {
            match key_event.code {
                crossterm::event::KeyCode::Char('q') => {
                    self.should_terminate.cancel();
                }
                crossterm::event::KeyCode::Char('l') => {
                    let cmd = rustctl_common::command::DownstreamClientMessage::ServerInstallOrUpdateAndStart;
                    let _ = self.tx_commands.send(cmd);
                }
                crossterm::event::KeyCode::Char('t') => {
                    let cmd = rustctl_common::command::DownstreamClientMessage::ServerSaveAndClose;
                    let _ = self.tx_commands.send(cmd);
                }
                _ => {}
            }
        }
    }

    impl Widget for &Ctl {
        fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
            use ratatui::layout::{Constraint, Direction, Layout};
            use ratatui::style::{Color, Modifier, Style};
            use ratatui::text::{Line, Span, Text};
            use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

            // Split into header and content
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(area);

            // Header
            let header_block = Block::default()
                .title(" rustctl Debug Client ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));

            let header_text = Text::from(vec![Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::Gray)),
                Span::styled("Q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(" to quit, ", Style::default().fg(Color::Gray)),
                Span::styled("L", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(" to launch, ", Style::default().fg(Color::Gray)),
                Span::styled("T", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" to terminate", Style::default().fg(Color::Gray)),
            ])]);

            Paragraph::new(header_text)
                .block(header_block)
                .render(chunks[0], buf);

            // Content area
            let content_block = Block::default()
                .title(" Snapshot Debug Output ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White));

            let content_text = if let Some(snapshot) = &self.latest_snapshot {
                let json_output = serde_json::to_string_pretty(snapshot).unwrap_or_else(|e| {
                    format!("Failed to serialize snapshot: {}", e)
                });
                Text::from(json_output)
            } else {
                Text::from(vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "Waiting for snapshot data...",
                        Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
                    )]),
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "WebSocket connection should be established automatically.",
                        Style::default().fg(Color::DarkGray),
                    )]),
                ])
            };

            Paragraph::new(content_text)
                .block(content_block)
                .wrap(Wrap { trim: false })
                .render(chunks[1], buf);
        }
    }
}
