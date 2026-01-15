//! Logging configuration
//!
//! Initializes tracing for the application.

/// Initializes logging with the specified level
pub fn init_logging(level: &str) {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging() {
        // Just verify it doesn't panic
        init_logging("debug");
    }
}
