use std::fs;
use std::path::Path;
use std::process::ExitStatus;

use crate::packages::LibraryCrate;
use crate::target::Target;
use crate::tools::Runner;
use crate::Result;

pub(crate) struct CargoRunner {
    runner: Runner,
}

impl CargoRunner {
    pub(crate) fn new() -> Result<Self> {
        let runner = Runner::new("cargo");
        Ok(Self { runner })
    }

    pub(crate) fn publish(&self, library_crate: &LibraryCrate) -> Result<()> {
        let package_name = library_crate.to_string();
        let target = None;
        match library_crate {
            LibraryCrate::ApolloFederationTypes => {
                self.cargo_exec(&["publish", "-p", &package_name], &[], target)?;
            }
            // Both crates need --allow-dirty because of the generated js files
            LibraryCrate::Harmonizer | LibraryCrate::RouterBridge => {
                self.cargo_exec(
                    &["publish", "-p", &package_name, "--allow-dirty"],
                    &[],
                    target,
                )?;
            }
        }
        Ok(())
    }

    // this function takes the cargo args, extra args, and optionally a target to run it for
    // targets can require _multiple_ invocations of cargo (notably universal macos)
    fn cargo_exec(
        &self,
        cargo_args: &[&str],
        extra_args: &[&str],
        target: Option<&Target>,
    ) -> Result<ExitStatus> {
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
        self.runner
            .exec(
                &cargo_args.iter().map(AsRef::as_ref).collect::<Vec<&str>>(),
                &[],
                env.as_ref(),
                false,
            )
            .map(|output| output.status)
    }
}

fn _copy_dir_all(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)?.flatten() {
        if let Ok(file_type) = entry.file_type() {
            if let Some(file_name) = entry.file_name().to_str() {
                let this_destination = destination.join(file_name);
                let this_source = entry.path();
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
