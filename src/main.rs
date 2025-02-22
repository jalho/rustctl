mod core;
mod game;
mod init;
mod system;

fn main() -> std::process::ExitCode {
    if let Err(exit) = init::logger() {
        return exit;
    }

    let expected = game::Resources::new();
    let initial_state = match crate::game::Game::check(&expected) {
        Ok(n) => n,
        Err(exit) => return exit,
    };

    let game = match initial_state.start() {
        Ok(n) => n,
        Err(exit) => return exit,
    };

    return game.wait();
}
