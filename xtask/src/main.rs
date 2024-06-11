mod commands;

pub(crate) mod packages;
pub(crate) mod target;
pub(crate) mod tools;

use ansi_term::Colour::Green;
use anyhow::Result;
use structopt::StructOpt;

fn main() -> Result<()> {
    simple_logger::init().unwrap();
    let app = Xtask::from_args();
    app.run()
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "xtask",
    about = "Workflows used locally and in CI for developing Rover"
)]
struct Xtask {
    #[structopt(subcommand)]
    pub command: Command,
}

#[derive(Debug, StructOpt)]
pub(crate) enum Command {
    /// Build federation-rs libraries for distribution.
    Dist(commands::Dist),

    /// Run linters for federation-rs libraries.
    Lint(commands::Lint),

    /// Run tests for federation-rs libraries.
    Test(commands::Test),

    /// This command should only ever be run in CI. Creates tarballs for binaries in the workspace.
    Package(commands::Package),

    /// This command should only ever be run in CI as you will need binaries from multiple platforms. You will just need to manually create the GitHub release from the `./artifacts` directory and create checksums. Publishes the crates in a given package group to crates.io and outputs binaries.
    Publish(commands::Publish),
}

impl Xtask {
    pub fn run(&self) -> Result<()> {
        match &self.command {
            Command::Dist(command) => command.run().map(|_| ()),
            Command::Lint(command) => command.run(),
            Command::Package(command) => command.run(),
            Command::Publish(command) => command.run(),
            Command::Test(command) => command.run(),
        }?;
        eprintln!("{}", Green.bold().paint("Success!"));
        Ok(())
    }
}
