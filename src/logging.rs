#[derive(clap::Args, Debug, Clone)]
#[group()]
pub struct LoggingArgs {
    /// Enable debug mode.
    #[arg(long, default_value_t = false)]
    pub debug: bool,

    /// Use Google cloud logging.
    #[cfg(feature = "gcp")]
    #[arg(long, default_value_t = false)]
    pub gcp_logging: bool,
}

impl LoggingArgs {
    pub fn init(&self) {
        #[cfg(feature = "gcp")]
        let gcp_logging = self.gcp_logging;

        #[cfg(not(feature = "gcp"))]
        let gcp_logging = false;
        init_logging(gcp_logging, self.debug);
    }
}

pub fn init_logging(gcp_logging: bool, debug_mode: bool) {
    #[cfg(feature = "gcp")]
    if gcp_logging {
        stackdriver_logger::init_with_cargo!("../Cargo.toml");
        return;
    }

    #[cfg(not(feature = "gcp"))]
    assert!(!gcp_logging, "GCP logging is not enabled");

    use log::LevelFilter;
    let level = if debug_mode {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    env_logger::builder().filter(None, level).init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_args_default() {
        let args = LoggingArgs {
            debug: false,
            #[cfg(feature = "gcp")]
            gcp_logging: false,
        };
        assert!(!args.debug);
        #[cfg(feature = "gcp")]
        assert!(!args.gcp_logging);
    }

    #[test]
    fn test_logging_args_debug() {
        let args = LoggingArgs {
            debug: true,
            #[cfg(feature = "gcp")]
            gcp_logging: false,
        };
        assert!(args.debug);
    }

    #[test]
    #[cfg(feature = "gcp")]
    fn test_logging_args_gcp() {
        let args = LoggingArgs {
            debug: false,
            gcp_logging: true,
        };
        assert!(args.gcp_logging);
    }

    #[test]
    #[should_panic(expected = "GCP logging is not enabled")]
    #[cfg(not(feature = "gcp"))]
    fn test_init_logging_gcp_without_feature() {
        init_logging(true, false);
    }

    #[test]
    #[cfg(not(feature = "gcp"))]
    fn test_init_logging_without_gcp() {
        // This should not panic since we're not requesting GCP logging
        init_logging(false, false);
        init_logging(false, true);
    }

    #[test]
    fn test_logging_args_clone() {
        let args = LoggingArgs {
            debug: true,
            #[cfg(feature = "gcp")]
            gcp_logging: false,
        };
        let cloned = args.clone();
        assert_eq!(args.debug, cloned.debug);
        #[cfg(feature = "gcp")]
        assert_eq!(args.gcp_logging, cloned.gcp_logging);
    }
}
