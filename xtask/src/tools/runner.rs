use anyhow::{Context, Result};

use log::info;
use std::collections::HashMap;
use std::process::{Command, Output};
use std::str;

pub(crate) struct Runner {
    pub(crate) bin: String,
}

impl Runner {
    pub(crate) fn new(bin: &str) -> Self {
        Runner {
            bin: bin.to_string(),
        }
    }
    pub(crate) fn exec(
        &self,
        args: &[&str],
        redacted_args: &[&str],
        env: Option<&HashMap<String, String>>,
    ) -> Result<Output> {
        info!("{bin} {args}", bin = &self.bin, args = args.join(" "));
        let mut task = Command::new(&self.bin);
        task.args(args);
        for redacted_arg in redacted_args {
            task.arg(redacted_arg);
        }
        if let Some(env) = env {
            for (k, v) in env {
                task.env(k, v);
            }
        }

        task.spawn()
            .context("Could not spawn process")?
            .wait_with_output()
            .context("Task did not complete")
    }
}
