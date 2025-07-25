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
            let title = ratatui::text::Line::from(ratatui::style::Stylize::bold(" rustctl "));

            let instructions = ratatui::text::Line::from(vec![
                " Quit ".into(),
                ratatui::style::Stylize::bold(ratatui::style::Stylize::blue("<Q>")),
                " Launch ".into(),
                ratatui::style::Stylize::bold(ratatui::style::Stylize::green("<L>")),
                " Terminate ".into(),
                ratatui::style::Stylize::bold(ratatui::style::Stylize::red("<T>")),
            ]);
            let block = ratatui::widgets::Block::bordered()
                .title(title.centered())
                .title_bottom(instructions.centered())
                .border_set(ratatui::symbols::border::THICK);

            let message_lines: Vec<ratatui::text::Line> = self
                .message_log
                .iter()
                .map(|msg| ratatui::text::Line::from(format!(" {}", msg.captured_at)))
                .collect();

            let message_text = ratatui::text::Text::from(message_lines);

            ratatui::widgets::Paragraph::new(message_text)
                .block(block)
                .render(area, buf);
        }
    }
}
