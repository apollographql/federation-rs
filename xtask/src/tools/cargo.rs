use anyhow::{anyhow, Context};
use camino::Utf8PathBuf;

use crate::packages::LibraryCrate;
use crate::target::Target;
use crate::tools::Runner;
use crate::utils::{self, CommandOutput};
use crate::Result;

use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;

pub(crate) struct CargoRunner {
    env: HashMap<String, String>,
    runner: Runner,
    workspace_roots: Vec<Utf8PathBuf>,
}

impl CargoRunner {
    pub(crate) fn new(verbose: bool) -> Result<Self> {
        let runner = Runner::new("cargo", verbose)?;
        let workspace_roots =
            utils::get_workspace_roots().context("Could not find one or more required packages")?;
        Ok(Self {
            env: HashMap::new(),
            runner,
            workspace_roots,
        })
    }

    pub(crate) fn lint(&self) -> Result<()> {
        self.run_all(vec!["fmt", "--all"], vec!["--check"])?;
        self.run_all(vec!["clippy", "--all"], vec!["-D", "warnings"])?;
        Ok(())
    }

    pub(crate) fn test(&self) -> Result<()> {
        let command_outputs =
            self.run_all(vec!["test", "--workspace", "--locked"], vec!["--nocapture"])?;

        // for some reason, cargo test doesn't actually fail if there are failed tests...????
        // so here we manually collect all the lines including failed tests and display them
        // as warnings for the dev.
        let mut failed_tests = Vec::new();
        for (command_output, directory) in command_outputs {
            for line in command_output.stdout.lines() {
                if line.starts_with("test") && line.contains("FAILED") {
                    failed_tests.push(line.to_string());
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
                    let _ = self.cargo_exec(
                        vec!["test"],
                        vec![exact_test, "--exact", "--nocapture"],
                        &directory,
                    );
                }
                Err(anyhow!("`cargo test` failed {} times.", failed_tests.len()))
            }?;
        }
        Ok(())
    }

    pub(crate) fn build(&mut self, target: &Target, release: bool) -> Result<()> {
        let mut cargo_args: Vec<String> = vec!["build", "--workspace"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        if release {
            cargo_args.push("--release".to_string());
            cargo_args.push("--locked".to_string());
        }
        let target_env = target.get_env()?;
        for (k, v) in target_env {
            self.env.insert(k, v);
        }
        cargo_args.extend(target.get_args());
        self.run_all(cargo_args.iter().map(|s| s.as_ref()).collect(), vec![])?;
        Ok(())
    }

    pub(crate) fn publish(
        &self,
        library_crate: &LibraryCrate,
        root_dir: &Utf8PathBuf,
    ) -> Result<()> {
        let package_name = library_crate.to_string();
        match library_crate {
            LibraryCrate::ApolloFederationTypes | LibraryCrate::RouterBridge => {
                self.cargo_exec(
                    vec!["publish", "--dry-run", "-p", &package_name],
                    vec![],
                    root_dir,
                )?;
                self.cargo_exec(vec!["publish", "-p", &package_name], vec![], root_dir)?;
            }
            LibraryCrate::Harmonizer => {
                self.cargo_exec(
                    vec!["publish", "--dry-run", "-p", &package_name, "--allow-dirty"],
                    vec![],
                    root_dir,
                )?;
                self.cargo_exec(
                    vec!["publish", "-p", &package_name, "--allow-dirty"],
                    vec![],
                    root_dir,
                )?;
            }
        }
        Ok(())
    }

    pub(crate) fn run_all(
        &self,
        cargo_args: Vec<&str>,
        extra_args: Vec<&str>,
    ) -> Result<Vec<(CommandOutput, Utf8PathBuf)>> {
        let mut command_outputs = Vec::new();
        for pkg_directory in &self.workspace_roots {
            let command_output = self
                .cargo_exec(cargo_args.clone(), extra_args.clone(), pkg_directory)
                .with_context(|| format!("Could not run command in `{}`", pkg_directory))?;
            command_outputs.push((command_output, pkg_directory.clone()));
        }
        Ok(command_outputs)
    }

    pub(crate) fn cargo_exec(
        &self,
        cargo_args: Vec<&str>,
        extra_args: Vec<&str>,
        directory: &Utf8PathBuf,
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
        self.runner.exec(&args, directory, env)
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
