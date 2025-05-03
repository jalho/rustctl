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
        tokio::spawn(system::monitor_usage(state.clone()));
        tokio::spawn(game::read_state(state.clone()));
        web::start(state).await;
    });
}
