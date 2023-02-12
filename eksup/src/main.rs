#![warn(missing_docs)]

//! `eksup` is a CLI to aid in upgrading Amazon EKS clusters

use anyhow::Result;
use clap::Parser;
use eksup::{analyze, create, Cli, Commands};

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();

  match &cli.commands {
    Commands::Analyze(args) => analyze(args).await?,
    Commands::Create(args) => create(args).await?,
  }

  Ok(())
}
