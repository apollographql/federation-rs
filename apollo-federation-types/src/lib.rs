#[cfg(feature = "build")]
pub mod build;

#[cfg(feature = "config")]
pub mod config;

pub(crate) type UncaughtJson = std::collections::BTreeMap<String, serde_json::Value>;
