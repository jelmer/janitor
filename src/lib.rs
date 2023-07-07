pub mod config;
pub mod state;

pub fn init_logging(gcp_logging: bool, debug_mode: bool) {
    if gcp_logging {
        stackdriver_logger::init_with_cargo!("../Cargo.toml");
    } else if debug_mode {
        env_logger::init();
    } else {
        env_logger::builder()
            .filter(None, log::LevelFilter::Info)
            .init();
    }
}
