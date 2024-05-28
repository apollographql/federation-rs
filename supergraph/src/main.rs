mod cli;
use cli::Supergraph;

pub(crate) mod command;

use structopt::StructOpt;

#[tokio::main]
async fn main() -> ! {
    let app = Supergraph::from_args();
    app.run().await
}
