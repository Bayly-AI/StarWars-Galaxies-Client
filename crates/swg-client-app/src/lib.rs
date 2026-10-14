// crates/swg-client-app/src/lib.rs

pub mod app;
pub mod config;

pub use app::{AppError, FrameStats, SwgClientApp};
pub use config::ClientConfig;
