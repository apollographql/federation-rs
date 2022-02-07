use crate::command::Command;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "supergraph",
    about = "A utility for working with Apollo Federation's supergraphs"
)]
pub struct Supergraph {
    #[structopt(subcommand)]
    command: Command,
}

impl Supergraph {
    pub fn run(&self) -> ! {
        match &self.command {
            Command::Compose(command) => command.run(),
        }
    }
}
