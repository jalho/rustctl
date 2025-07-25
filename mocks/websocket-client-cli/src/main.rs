fn main() -> std::process::ExitCode {
    let coroutine_tui: std::thread::JoinHandle<_> = std::thread::spawn(tui::work);

    let _tui_done: () = coroutine_tui.join().unwrap();
    ratatui::restore();

    std::process::ExitCode::SUCCESS
}

mod tui {
    pub fn work() {
        let mut terminal: ratatui::Terminal<_> = ratatui::init();
        let _app_done = App::new().run(&mut terminal).unwrap();
    }

    pub struct App {
        messages_received: u8,
        should_terminate: bool,
    }

    impl App {
        pub fn new() -> Self {
            Self {
                messages_received: 1,
                should_terminate: false,
            }
        }

        pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
            while !self.should_terminate {
                terminal.draw(|frame| self.draw(frame))?;
                self.handle_events()?;
            }
            Ok(())
        }

        fn draw(&self, frame: &mut ratatui::Frame) {
            frame.render_widget(self, frame.area());
        }

        fn handle_events(&mut self) -> std::io::Result<()> {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key_event)
                    if key_event.kind == crossterm::event::KeyEventKind::Press =>
                {
                    self.handle_key_event(key_event)
                }
                _ => {}
            };
            Ok(())
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

    impl ratatui::widgets::Widget for &App {
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
