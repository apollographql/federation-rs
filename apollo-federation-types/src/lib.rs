#[cfg(feature = "build")]
pub mod build;

#[cfg(feature = "build_plugin")]
pub mod build_plugin;

#[cfg(feature = "config")]
pub mod config;

pub(crate) type UncaughtJson = std::collections::BTreeMap<String, serde_json::Value>;
