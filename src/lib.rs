pub mod adapters;
pub mod config;
pub mod domain;
pub mod telemetry;

pub use adapters::{
    inbound::{cli::run_healthcheck, http::create_router},
    outbound::InMemoryItemRepository,
};
pub use config::AppConfig;
