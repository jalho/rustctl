fn main() -> std::process::ExitCode {
    let (tx_updates, rx_updates) = std::sync::mpsc::channel::<String>();

    let th_tui = std::thread::spawn(|| tui::work(rx_updates));
    let th_connection = std::thread::spawn(|| connection::work(tx_updates));

    let _done_tui: () = th_tui.join().unwrap();
    let _done_connection: () = th_connection.join().unwrap();

    ratatui::restore();
    std::process::ExitCode::SUCCESS
}

mod connection {
    pub fn work(tx_updates: std::sync::mpsc::Sender<String>) {
        let rt: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        let _coroutine_done = rt.block_on(connect(tx_updates));
    }

    async fn connect(tx_updates: std::sync::mpsc::Sender<String>) {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            tx_updates.send("Slept a bit!".into()).unwrap();
        }
    }
}

mod tui {
    pub fn work(rx_updates: std::sync::mpsc::Receiver<String>) {
        let mut terminal: ratatui::Terminal<_> = ratatui::init();
        let _app_done = Ctl::new(rx_updates).run(&mut terminal).unwrap();
    }

    pub struct Ctl {
        should_terminate: bool,

        rx_updates: std::sync::mpsc::Receiver<String>,

        messages_received: u8,
    }

    impl Ctl {
        pub fn new(rx_updates: std::sync::mpsc::Receiver<String>) -> Self {
            Self {
                should_terminate: false,
                rx_updates,
                messages_received: 1,
            }
        }

        pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
            while !self.should_terminate {
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
            match key_event.code {
                crossterm::event::KeyCode::Char('q') => self.app_quit(),
                crossterm::event::KeyCode::Char('l') => self.cmd_launch_game(),
                crossterm::event::KeyCode::Char('t') => self.cmd_terminate_game(),
                _ => {}
            }
        }

        fn app_quit(&mut self) {
            self.should_terminate = true;
        }

        fn cmd_terminate_game(&mut self) {
            self.messages_received += 1;
        }

        fn cmd_launch_game(&mut self) {
            self.messages_received -= 1;
        }
    }

    impl ratatui::widgets::Widget for &Ctl {
        fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
            let title = ratatui::text::Line::from(ratatui::style::Stylize::bold(" rustctl "));

            let instructions = ratatui::text::Line::from(vec![
                " Launch ".into(),
                ratatui::style::Stylize::bold(ratatui::style::Stylize::blue("<L>")),
                " Terminate ".into(),
                ratatui::style::Stylize::bold(ratatui::style::Stylize::blue("<T>")),
                " Quit ".into(),
                ratatui::style::Stylize::bold(ratatui::style::Stylize::blue("<Q>")),
            ]);
            let block = ratatui::widgets::Block::bordered()
                .title(title.centered())
                .title_bottom(instructions.centered())
                .border_set(ratatui::symbols::border::THICK);

            let counter_text = ratatui::text::Text::from(vec![ratatui::text::Line::from(vec![
                "Value: ".into(),
                ratatui::style::Stylize::yellow(self.messages_received.to_string()),
            ])]);

            ratatui::widgets::Paragraph::new(counter_text)
                .centered()
                .block(block)
                .render(area, buf);
        }
    }
}
