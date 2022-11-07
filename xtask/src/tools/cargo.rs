use anyhow::{anyhow, Context};
use camino::Utf8PathBuf;

use crate::packages::LibraryCrate;
use crate::target::Target;
use crate::tools::Runner;
use crate::utils::{self, CommandOutput};
use crate::Result;

use std::convert::TryInto;
use std::fs;

pub(crate) struct CargoRunner {
    runner: Runner,
    workspace_roots: Vec<Utf8PathBuf>,
}

impl CargoRunner {
    pub(crate) fn new(verbose: bool) -> Result<Self> {
        let runner = Runner::new("cargo", verbose)?;
        let workspace_roots =
            utils::get_workspace_roots().context("Could not find one or more required packages")?;
        Ok(Self {
            runner,
            workspace_roots,
        })
    }

    pub(crate) fn lint_all(&self) -> Result<()> {
        for workspace_root in &self.workspace_roots {
            self.lint(workspace_root)?;
        }
        Ok(())
    }

    pub(crate) fn lint(&self, workspace_directory: &Utf8PathBuf) -> Result<()> {
        let target = None;
        self.cargo_exec(
            vec!["fmt", "--all"],
            vec!["--check"],
            target,
            workspace_directory,
        )?;
        self.cargo_exec(
            vec!["clippy"],
            vec!["-D", "warnings"],
            target,
            workspace_directory,
        )?;
        Ok(())
    }

    pub(crate) fn test_all(&self) -> Result<()> {
        for workspace_root in &self.workspace_roots {
            self.test(workspace_root)?;
        }
        Ok(())
    }

    pub(crate) fn test(&self, workspace_directory: &Utf8PathBuf) -> Result<()> {
        let target = None;
        let command_output = self.cargo_exec(
            vec!["test", "--locked"],
            vec!["--nocapture"],
            target,
            workspace_directory,
        )?;

        // for some reason, cargo test doesn't actually fail if there are failed tests...????
        // so here we manually collect all the lines including failed tests and display them
        // as warnings for the dev.
        let mut failed_tests = Vec::new();

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
                    target,
                    workspace_directory,
                );
            }
            Err(anyhow!("`cargo test` failed {} times.", failed_tests.len()))
        }
    }

    pub(crate) fn build(
        &self,
        target: &Target,
        release: bool,
        workspace_directory: &Utf8PathBuf,
    ) -> Result<()> {
        let mut cargo_args: Vec<&str> = vec!["build"];
        if release {
            cargo_args.push("--release");
            cargo_args.push("--locked");
        }
        self.cargo_exec(cargo_args, vec![], Some(target), workspace_directory)?;
        crate::info!(
            "successfully compiled all packages in workspace root `{}`",
            &workspace_directory
        );
        Ok(())
    }

    pub(crate) fn build_all(&self, target: &Target, release: bool) -> Result<()> {
        for workspace_root in self.workspace_roots.clone() {
            self.build(target, release, &workspace_root)?;
        }
        Ok(())
    }

    pub(crate) fn publish(
        &self,
        library_crate: &LibraryCrate,
        workspace_directory: &Utf8PathBuf,
    ) -> Result<()> {
        let package_name = library_crate.to_string();
        let target = None;
        match library_crate {
            LibraryCrate::ApolloFederationTypes => {
                self.cargo_exec(
                    vec!["publish", "-p", &package_name],
                    vec![],
                    target,
                    workspace_directory,
                )?;
            }
            // Both crates need --allow-dirty because of the generated js files
            LibraryCrate::Harmonizer | LibraryCrate::RouterBridge => {
                self.cargo_exec(
                    vec!["publish", "-p", &package_name, "--allow-dirty"],
                    vec![],
                    target,
                    workspace_directory,
                )?;
            }
        }
        Ok(())
    }

    // this function takes the cargo args, extra args, and optionally a target to run it for
    // targets can require _multiple_ invocations of cargo (notably universal macos)
    fn cargo_exec(
        &self,
        cargo_args: Vec<&str>,
        extra_args: Vec<&str>,
        target: Option<&Target>,
        directory: &Utf8PathBuf,
    ) -> Result<CommandOutput> {
        let mut cargo_args = cargo_args
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        if !extra_args.is_empty() {
            cargo_args.push("--".to_string());
            for extra_arg in extra_args {
                cargo_args.push(extra_arg.to_string());
            }
        }
        let mut env = None;
        if let Some(target) = target {
            cargo_args.extend(target.get_cargo_args());
            env = Some(target.get_env()?);
        };
        self.runner.exec(
            &cargo_args.iter().map(AsRef::as_ref).collect::<Vec<&str>>(),
            directory,
            env.as_ref(),
        )
    }
}

fn _copy_dir_all(source: &Utf8PathBuf, destination: &Utf8PathBuf) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)?.flatten() {
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
