mod core;
mod error;
mod game;
mod init;
mod system;

fn main() -> std::process::ExitCode {
    if let Err(exit) = init::logger() {
        return exit;
    }

    let initial_state = match crate::game::Game::check() {
        Ok(n) => n,
        Err(exit) => return exit,
    };

    let game = match initial_state.start() {
        Ok(n) => n,
        Err(exit) => return exit,
    };

    return game.wait();
}
