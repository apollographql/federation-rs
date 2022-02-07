mod cli;
use cli::Supergraph;

pub(crate) mod command;

use structopt::StructOpt;

fn main() -> ! {
    let app = Supergraph::from_args();
    app.run();
}
