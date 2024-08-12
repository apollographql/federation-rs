use std::{collections::HashMap, fmt, str::FromStr};

use crate::Result;

pub(crate) const TARGET_LINUX_UNKNOWN_GNU: &str = "x86_64-unknown-linux-gnu";
pub(crate) const TARGET_LINUX_UNKNOWN_MUSL: &str = "x86_64-unknown-linux-musl";
pub(crate) const TARGET_LINUX_ARM: &str = "aarch64-unknown-linux-gnu";
pub(crate) const TARGET_WINDOWS_MSVC: &str = "x86_64-pc-windows-msvc";
pub(crate) const TARGET_MACOS_INTEL: &str = "x86_64-apple-darwin";
pub(crate) const TARGET_MACOS_ARM: &str = "aarch64-apple-darwin";

pub(crate) const POSSIBLE_TARGETS: [&str; 5] = [
    TARGET_LINUX_UNKNOWN_GNU,
    TARGET_LINUX_ARM,
    TARGET_WINDOWS_MSVC,
    TARGET_MACOS_INTEL,
    TARGET_MACOS_ARM,
    TARGET_LINUX_UNKNOWN_MUSL,
];

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Target {
    LinuxUnknownGnu,
    LinuxUnknownMusl,
    LinuxAarch,
    WindowsMsvc,
    MacOSIntel,
    MacOSArm,
    Other,
}

impl Target {
    pub(crate) fn get_cargo_args(&self) -> Vec<String> {
        let mut target_args = Vec::new();
        if !self.is_other() {
            target_args.push("--target".to_string());
            target_args.push(self.to_string());
        }
        target_args
    }

    pub(crate) fn is_other(&self) -> bool {
        Self::Other == *self
    }

    #[allow(unused)]
    pub(crate) fn is_macos(&self) -> bool {
        Self::MacOSIntel == *self || Self::MacOSArm == *self
    }

    #[allow(unused)]
    pub(crate) fn is_linux(&self) -> bool {
        Self::LinuxAarch == *self || Self::LinuxUnknownGnu == *self || Self::LinuxUnknownMusl == *self
    }

    #[allow(unused)]
    pub(crate) fn is_musl(&self) -> bool {
        Self::LinuxUnknownMusl == *self
    }

    pub(crate) fn is_windows(&self) -> bool {
        Self::WindowsMsvc == *self
    }

    pub(crate) fn get_env(&self) -> Result<HashMap<String, String>> {
        let mut env = HashMap::new();
        if self.is_windows() {
            env.insert(
                "RUSTFLAGS".to_string(),
                "-Ctarget-feature=+crt-static".to_string(),
            );
        }
        if self.is_musl() {
            env.insert(
                "V8_FROM_SOURCE".to_string(), true.to_string()
            )
        }
        Ok(env)
    }
}

impl Default for Target {
    fn default() -> Self {
        let mut result = Target::Other;
        if cfg!(target_os = "windows") {
            if cfg!(target_arch = "x86_64") {
                result = Target::WindowsMsvc;
            }
        } else if cfg!(target_os = "linux") {
            if cfg!(target_env = "gnu") {
                if cfg!(target_arch = "x86_64") {
                    result = Target::LinuxUnknownGnu
                } else if cfg!(target_arch = "aarch64") {
                    result = Target::LinuxAarch
                }
            }
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "x86_64") {
                result = Target::MacOSIntel
            } else if cfg!(target_arch = "aarch64") {
                result = Target::MacOSArm
            }
        }
        result
    }
}

impl FromStr for Target {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            TARGET_LINUX_UNKNOWN_GNU => Ok(Self::LinuxUnknownGnu),
            TARGET_LINUX_ARM => Ok(Self::LinuxAarch),
            TARGET_WINDOWS_MSVC => Ok(Self::WindowsMsvc),
            TARGET_MACOS_INTEL => Ok(Self::MacOSIntel),
            TARGET_MACOS_ARM => Ok(Self::MacOSArm),
            _ => Ok(Self::Other),
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Target::LinuxUnknownGnu => TARGET_LINUX_UNKNOWN_GNU,
            Target::LinuxAarch => TARGET_LINUX_ARM,
            Target::WindowsMsvc => TARGET_WINDOWS_MSVC,
            Target::MacOSIntel => TARGET_MACOS_INTEL,
            Target::MacOSArm => TARGET_MACOS_ARM,
            Target::Other => "unknown-target",
        };
        write!(f, "{msg}")
    }
}
