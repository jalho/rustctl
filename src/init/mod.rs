use log4rs::{
    append::console::ConsoleAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};

pub fn logger() -> Result<log4rs::Handle, std::process::ExitCode> {
    let pattern = "{date(%H:%M:%S)} [{thread}] {highlight({({message})})}{n}";
    let encoder = PatternEncoder::new(pattern);

    let console_appender_builder = ConsoleAppender::builder();
    let console_appender = console_appender_builder.encoder(Box::new(encoder)).build();

    let appender_builder = Appender::builder();
    let appender_name = "stdout";
    let appender = appender_builder.build(appender_name, Box::new(console_appender));

    let root_builder = Root::builder().appender(appender_name);
    let root = root_builder.build(log::LevelFilter::max());

    let config_builder = Config::builder();
    let config = match config_builder.appender(appender).build(root) {
        Ok(n) => n,
        Err(err) => {
            eprintln!("cannot configure logger: {err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    };

    match log4rs::init_config(config) {
        Ok(handle) => {
            log::debug!("Logger initialized");
            return Ok(handle);
        }
        Err(err) => {
            eprintln!("cannot initialize logger: {err}");
            return Err(std::process::ExitCode::FAILURE);
        }
    }
}
