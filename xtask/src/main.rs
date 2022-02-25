mod commands;

pub(crate) mod packages;
pub(crate) mod tools;
pub(crate) mod utils;

use ansi_term::Colour::Green;
use anyhow::Result;
use structopt::StructOpt;

fn main() -> Result<()> {
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

    /// Specify xtask's verbosity level
    #[structopt(long = "verbose", short = "v", global = true)]
    verbose: bool,
}

#[derive(Debug, StructOpt)]
pub(crate) enum Command {
    /// Build federation-rs libraries for distribution
    Dist(commands::Dist),

    /// Run linters for federation-rs libraries
    Lint(commands::Lint),

    /// Prep federation-rs libraries for release
    Prep(commands::Prep),

    /// Run tests for federation-rs libraries
    Test(commands::Test),

    /// Creates a release tag for a given package group from the main branch
    Tag(commands::Tag),

    /// Publishes a release for a given package tag from the main branch
    Publish(commands::Publish),
}

impl Xtask {
    pub fn run(&self) -> Result<()> {
        match &self.command {
            Command::Dist(command) => command.run(self.verbose),
            Command::Lint(command) => command.run(self.verbose),
            Command::Prep(command) => command.run(self.verbose).map(|_| ()),
            Command::Publish(command) => command.run(self.verbose),
            Command::Tag(command) => command.run(self.verbose),
            Command::Test(command) => command.run(self.verbose),
        }?;
        eprintln!("{}", Green.bold().paint("Success!"));
        Ok(())
    }
}
