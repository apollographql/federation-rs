#[cfg(feature = "build")]
pub mod rover;

#[cfg(feature = "build_plugin")]
pub mod build_plugin;

#[cfg(feature = "config")]
pub mod config;

#[cfg(feature = "composition")]
pub mod composition;
pub mod javascript;

pub(crate) type UncaughtJson = std::collections::BTreeMap<String, serde_json::Value>;
