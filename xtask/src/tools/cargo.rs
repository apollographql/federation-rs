use anyhow::anyhow;
use camino::Utf8PathBuf;

use crate::target::Target;
use crate::tools::Runner;
use crate::utils::{CommandOutput, PKG_PROJECT_ROOT};
use crate::Result;

use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;

pub(crate) struct CargoRunner {
    cargo_package_directory: Utf8PathBuf,
    runner: Runner,
    env: HashMap<String, String>,
}

impl CargoRunner {
    pub(crate) fn new(verbose: bool) -> Result<Self> {
        Self::new_with_path(verbose, PKG_PROJECT_ROOT.clone())
    }

    pub(crate) fn new_with_path<P>(verbose: bool, cargo_package_directory: P) -> Result<Self>
    where
        P: Into<Utf8PathBuf>,
    {
        let runner = Runner::new("cargo", verbose)?;
        Ok(CargoRunner {
            cargo_package_directory: cargo_package_directory.into(),
            runner,
            env: HashMap::new(),
        })
    }

    pub(crate) fn lint(&self) -> Result<()> {
        self.cargo_exec(vec!["fmt", "--all"], vec!["--check"])?;
        self.cargo_exec(vec!["clippy", "--all"], vec!["-D", "warnings"])?;
        Ok(())
    }

    pub(crate) fn test(&self) -> Result<()> {
        let command_output =
            self.cargo_exec(vec!["test", "--workspace", "--locked"], vec!["--nocapture"])?;

        // for some reason, cargo test doesn't actually fail if there are failed tests...????
        // so here we manually collect all the lines including failed tests and display them
        // as warnings for the dev.
        let mut failed_tests = Vec::new();

        for line in command_output.stdout.lines() {
            if line.starts_with("test") && line.contains("FAILED") {
                failed_tests.push(line);
            }
        }

        if failed_tests.is_empty() {
            Ok(())
        } else {
            for failed_test in &failed_tests {
                let split_test: Vec<&str> = failed_test.splitn(3, ' ').collect();
                if split_test.len() < 3 {
                    panic!("Something went wrong with xtask's failed test detection.");
                }
                let exact_test = split_test[1];

                // drop the result here so we can re-run the failed tests and print their output.
                let _ = self.cargo_exec(vec!["test"], vec![exact_test, "--exact", "--nocapture"]);
            }
            Err(anyhow!("`cargo test` failed {} times.", failed_tests.len()))
        }
    }

    pub(crate) fn build(&mut self, target: &Target, release: bool) -> Result<()> {
        let mut cargo_args: Vec<String> = vec!["build", "--workspace"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        if release {
            cargo_args.push("--release".to_string());
        }
        if self.cargo_package_directory.ends_with("stage") {
            // since we do weird stuff to our workspace, the lockfile is a _bit_ unhappy. but since we know we
            // just copied it, it's safe to use --offline here.
            cargo_args.push("--offline".to_string())
        } else {
            cargo_args.push("--locked".to_string())
        }
        let target_env = target.get_env()?;
        for (k, v) in target_env {
            self.env.insert(k, v);
        }
        cargo_args.extend(target.get_args());
        self.cargo_exec(cargo_args.iter().map(|s| s.as_ref()).collect(), vec![])?;
        Ok(())
    }

    pub(crate) fn publish(&self, package_name: &str) -> Result<()> {
        self.cargo_exec(vec!["publish", "--dry-run", "-p", package_name], vec![])?;
        self.cargo_exec(vec!["publish", "-p", &package_name], vec![])?;
        Ok(())
    }

    pub(crate) fn cargo_exec(
        &self,
        cargo_args: Vec<&str>,
        extra_args: Vec<&str>,
    ) -> Result<CommandOutput> {
        let mut args = cargo_args;
        if !extra_args.is_empty() {
            args.push("--");
            for extra_arg in extra_args {
                args.push(extra_arg);
            }
        }
        let env = if self.env.is_empty() {
            None
        } else {
            Some(&self.env)
        };
        self.runner.exec(&args, &self.cargo_package_directory, env)
    }
}

fn _copy_dir_all(source: &Utf8PathBuf, destination: &Utf8PathBuf) -> Result<()> {
    fs::create_dir_all(&destination)?;
    for entry in fs::read_dir(&source)?.flatten() {
        if let Ok(file_type) = entry.file_type() {
            if let Some(file_name) = entry.file_name().to_str() {
                let this_destination = destination.join(file_name);
                let this_source = entry.path().try_into()?;
                if file_type.is_dir() {
                    _copy_dir_all(&this_source, &this_destination)?;
                } else {
                    fs::copy(this_source, this_destination)?;
                }
            }
        }
    }
    Ok(())
}
