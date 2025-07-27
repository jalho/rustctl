fn main() -> std::process::ExitCode {
    let c0: tokio_util::sync::CancellationToken = tokio_util::sync::CancellationToken::new();
    let c1: tokio_util::sync::CancellationToken = c0.child_token();

    let (tx_updates, rx_updates) = std::sync::mpsc::channel::<rustctl_common::snapshot::Snapshot>();
    let (tx_commands, rx_commands) =
        std::sync::mpsc::channel::<rustctl_common::command::DownstreamClientMessage>();

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
        let (stream, _response) = tokio_tungstenite::connect_async(format!(
            "ws://127.0.0.1:8080{WEBSOCKET_CONNECT_URL_PATH}"
        ))
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

            let deserialized: rustctl_common::snapshot::Snapshot =
                serde_json::from_str(&utf8).unwrap();

            if tx_updates.send(deserialized).is_err() {
                break 'recv_messages;
            }
        }

        coroutine_pass_commands.abort();
    }
}

mod tui {
    use ratatui::prelude::Widget;

    const MSG_STORE_SIZE: usize = 4;

    pub fn work(
        rx_updates: std::sync::mpsc::Receiver<rustctl_common::snapshot::Snapshot>,
        tx_commands: std::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
        cancel: tokio_util::sync::CancellationToken,
    ) {
        let mut terminal: ratatui::Terminal<_> = ratatui::init();
        let _app_done = Ctl::new(rx_updates, tx_commands, cancel)
            .run(&mut terminal)
            .unwrap();
    }

    pub struct Ctl {
        should_terminate: tokio_util::sync::CancellationToken,
        rx_updates: std::sync::mpsc::Receiver<rustctl_common::snapshot::Snapshot>,
        tx_commands: std::sync::mpsc::Sender<rustctl_common::command::DownstreamClientMessage>,
        message_log: std::collections::VecDeque<rustctl_common::snapshot::Snapshot>,
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
                message_log: std::collections::VecDeque::with_capacity(MSG_STORE_SIZE),
            }
        }

        pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
            while !self.should_terminate.is_cancelled() {
                while let Ok(msg) = self.rx_updates.try_recv() {
                    if self.message_log.len() >= self.message_log.capacity() {
                        self.message_log.pop_front();
                    }
                    self.message_log.push_back(msg);
                }

                terminal.draw(|frame| self.draw(frame))?;

                if crossterm::event::poll(std::time::Duration::from_millis(100))? {
                    let key_event = crossterm::event::read()?;
                    match key_event {
                        crossterm::event::Event::Key(key_event) => self.handle_key_event(key_event),
                        _ => {}
                    }
                }
            }
            Ok(())
        }

        fn draw(&self, frame: &mut ratatui::Frame) {
            frame.render_widget(self, frame.area());
        }

        fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) {
            let cmd_launch: rustctl_common::command::DownstreamClientMessage =
                rustctl_common::command::DownstreamClientMessage::ServerInstallOrUpdateAndStart;

            let cmd_terminate: rustctl_common::command::DownstreamClientMessage =
                rustctl_common::command::DownstreamClientMessage::ServerSaveAndClose;

            match key_event.code {
                crossterm::event::KeyCode::Char('q') => self.app_quit(),
                crossterm::event::KeyCode::Char('l') => {
                    let _ = self.tx_commands.send(cmd_launch);
                }
                crossterm::event::KeyCode::Char('t') => {
                    let _ = self.tx_commands.send(cmd_terminate);
                }
                _ => {}
            }
        }

        fn app_quit(&mut self) {
            self.should_terminate.cancel();
        }
    }

    impl ratatui::widgets::Widget for &Ctl {
        fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
            use ratatui::layout::{Alignment, Constraint, Direction, Layout};
            use ratatui::style::{Color, Modifier, Style};
            use ratatui::symbols::border::ROUNDED;
            use ratatui::text::{Line, Span, Text};
            use ratatui::widgets::{Block, Paragraph};

            // Main layout: header + dashboard + footer
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(0),    // Dashboard content
                    Constraint::Length(3), // Footer
                ])
                .split(area);

            // Render header
            let header_title = Line::from(vec![
                Span::styled("🦀 ", Style::default().fg(Color::Red)),
                Span::styled(
                    "rustctl",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Dashboard", Style::default().fg(Color::White)),
            ]);

            let header_block = Block::bordered()
                .title(header_title.centered())
                .border_set(ROUNDED)
                .border_style(Style::default().fg(Color::Cyan));

            Paragraph::new("")
                .block(header_block)
                .render(main_chunks[0], buf);

            // Render footer with controls
            let instructions = Line::from(vec![
                Span::styled("❌ ", Style::default().fg(Color::Red)),
                Span::styled("Quit", Style::default().fg(Color::White)),
                Span::styled(" [", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "Q",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled("] ", Style::default().fg(Color::DarkGray)),
                Span::styled("  🚀 ", Style::default().fg(Color::Green)),
                Span::styled("Launch", Style::default().fg(Color::White)),
                Span::styled(" [", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "L",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("] ", Style::default().fg(Color::DarkGray)),
                Span::styled("  ⛔ ", Style::default().fg(Color::Red)),
                Span::styled("Terminate", Style::default().fg(Color::White)),
                Span::styled(" [", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "T",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled("]", Style::default().fg(Color::DarkGray)),
            ]);

            let footer_block = Block::bordered()
                .title_bottom(instructions.centered())
                .border_set(ROUNDED)
                .border_style(Style::default().fg(Color::Gray));

            Paragraph::new("")
                .block(footer_block)
                .render(main_chunks[2], buf);

            // Dashboard content area
            let content_area = main_chunks[1];

            if self.message_log.is_empty() {
                // Empty state
                let empty_block = Block::bordered()
                    .title(Line::from(vec![
                        Span::styled("📊 ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            "System Metrics",
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]))
                    .border_set(ROUNDED)
                    .border_style(Style::default().fg(Color::DarkGray));

                let empty_text = Text::from(vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("🔍 ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            "No system metrics available yet...",
                            Style::default()
                                .fg(Color::Gray)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("⏳ ", Style::default().fg(Color::Blue)),
                        Span::styled(
                            "Waiting for data collection to begin",
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]),
                ]);

                Paragraph::new(empty_text)
                    .block(empty_block)
                    .alignment(Alignment::Center)
                    .render(content_area, buf);
                return;
            }

            // Create grid layout for metric cards (2 columns)
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .margin(1)
                .split(content_area);

            // Left column: Memory metrics
            self.render_memory_column(&cols[0], buf);

            // Right column: CPU metrics
            self.render_cpu_column(&cols[1], buf);
        }
    }

    impl Ctl {
        fn render_memory_column(
            &self,
            area: &ratatui::layout::Rect,
            buf: &mut ratatui::buffer::Buffer,
        ) {
            use ratatui::layout::Alignment;
            use ratatui::style::{Color, Modifier, Style};
            use ratatui::symbols::border::ROUNDED;
            use ratatui::text::{Line, Span, Text};
            use ratatui::widgets::{Block, Paragraph};

            // Create memory block
            let memory_block = Block::bordered()
                .title(Line::from(vec![
                    Span::styled("📊 ", Style::default().fg(Color::LightBlue)),
                    Span::styled(
                        "Memory Usage",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .border_set(ROUNDED)
                .border_style(Style::default().fg(Color::Blue));

            let inner_area = memory_block.inner(*area);

            // Render block first
            memory_block.render(*area, buf);

            // Create content for each snapshot
            let mut content_lines = Vec::new();

            for (idx, snapshot) in self.message_log.iter().rev().enumerate() {
                let mem_timestamp = &snapshot.system_memory_usage_total.read_completed_by;
                let mem_value = &snapshot.system_memory_usage_total.read_value;

                // Format timestamp
                let timestamp_str = mem_timestamp.format("%H:%M:%S").to_string();

                let accent_color = if idx % 2 == 0 {
                    Color::LightBlue
                } else {
                    Color::LightCyan
                };

                content_lines.push(Line::from("")); // Spacing

                // Timestamp line
                content_lines.push(Line::from(vec![
                    Span::styled("⏰ ", Style::default().fg(Color::Yellow)),
                    Span::styled(timestamp_str, Style::default().fg(Color::DarkGray)),
                ]));

                // Memory usage line - display the actual memory value
                content_lines.push(Line::from(vec![
                    Span::styled("💾 ", Style::default().fg(accent_color)),
                    Span::styled(
                        format!("{}", mem_value),
                        Style::default()
                            .fg(accent_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }

            if content_lines.is_empty() {
                content_lines.push(Line::from(vec![
                    Span::styled("📭 ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        "No memory data",
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
            }

            let memory_text = Text::from(content_lines);
            Paragraph::new(memory_text)
                .alignment(Alignment::Left)
                .render(inner_area, buf);
        }

        fn render_cpu_column(
            &self,
            area: &ratatui::layout::Rect,
            buf: &mut ratatui::buffer::Buffer,
        ) {
            use ratatui::layout::Alignment;
            use ratatui::style::{Color, Modifier, Style};
            use ratatui::symbols::border::ROUNDED;
            use ratatui::text::{Line, Span, Text};
            use ratatui::widgets::{Block, Paragraph};

            // Create CPU block
            let cpu_block = Block::bordered()
                .title(Line::from(vec![
                    Span::styled("🖥️ ", Style::default().fg(Color::LightGreen)),
                    Span::styled(
                        "CPU Usage",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .border_set(ROUNDED)
                .border_style(Style::default().fg(Color::Green));

            let inner_area = cpu_block.inner(*area);

            // Render block first
            cpu_block.render(*area, buf);

            // Create content for each snapshot
            let mut content_lines = Vec::new();

            for (idx, snapshot) in self.message_log.iter().rev().enumerate() {
                let cpu_timestamp = &snapshot.system_cpu_usage_total.read_completed_by;
                let cpu_values: &Vec<rustctl_common::snapshot::CpuUsage> =
                    &snapshot.system_cpu_usage_total.read_value;

                // Format timestamp
                let timestamp_str = cpu_timestamp.format("%H:%M:%S").to_string();

                // Create alternating colors
                let accent_color = if idx % 2 == 0 {
                    Color::LightGreen
                } else {
                    Color::LightYellow
                };

                content_lines.push(Line::from("")); // Spacing

                // Timestamp line
                content_lines.push(Line::from(vec![
                    Span::styled("⏰ ", Style::default().fg(Color::Yellow)),
                    Span::styled(timestamp_str, Style::default().fg(Color::DarkGray)),
                ]));

                // Display each CPU's usage individually
                for (cpu_idx, cpu_value) in cpu_values.iter().enumerate() {
                    // Choose color based on usage level
                    let usage_color = match cpu_value.as_percentage() {
                        p if p >= 80.0 => Color::Red,
                        p if p >= 60.0 => Color::Yellow,
                        p if p >= 40.0 => Color::LightYellow,
                        _ => accent_color,
                    };

                    content_lines.push(Line::from(vec![
                        Span::styled("⚡ ", Style::default().fg(accent_color)),
                        Span::styled(
                            format!("CPU{}: ", cpu_idx),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!("{:.1}%", cpu_value.as_percentage()),
                            Style::default()
                                .fg(usage_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }
            }

            if content_lines.is_empty() {
                content_lines.push(Line::from(vec![
                    Span::styled("📭 ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        "No CPU data",
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
            }

            let cpu_text = Text::from(content_lines);
            Paragraph::new(cpu_text)
                .alignment(Alignment::Left)
                .render(inner_area, buf);
        }
    }
}
