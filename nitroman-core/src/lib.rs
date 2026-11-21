pub mod config;
pub mod driver;
pub mod error;
pub mod model;
pub mod nvml;
pub mod sensors;
pub mod specs;
pub mod wmi;

pub use config::UserConfig;
pub use driver::AcerDriver;
pub use error::{Error, Result};
pub use model::*;
pub use sensors::Telemetry;
pub use specs::SystemSpecs;
