#![warn(missing_docs)]

//! `eksup` is a CLI to aid in upgrading Amazon EKS clusters

use anyhow::Result;
use clap::Parser;
use eksup::{Cli, Commands, analyze, create};
use tracing_log::AsTrace;
use tracing_subscriber::FmtSubscriber;

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();

  let subscriber = FmtSubscriber::builder()
    .with_max_level(cli.verbose.log_level_filter().as_trace())
    .without_time()
    .pretty()
    .finish();
  tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

  match cli.commands {
    Commands::Analyze(args) => analyze(args).await?,
    Commands::Create(args) => create(args).await?,
  }

  Ok(())
}
