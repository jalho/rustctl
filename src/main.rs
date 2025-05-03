mod constants;
mod core;
mod game;
mod system;
mod web;

fn main() {
    let state = core::SharedState::init();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        /*
         * Monitor system resources's usage such as CPU and memory.
         */
        tokio::spawn(system::monitor_usage(state.clone()));

        /*
         * Read game state such as players's locations.
         */
        tokio::spawn(game::read_state(state.clone()));

        /*
         * Serve a web app for observing and managing the system.
         */
        web::start(state).await;
    });
}
