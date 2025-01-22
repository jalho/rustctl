mod misc;

fn main() -> std::process::ExitCode {
    crate::misc::init_logger();
    return std::process::ExitCode::SUCCESS;
}
