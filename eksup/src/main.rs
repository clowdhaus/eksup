#![warn(missing_docs)]

//! `eksup` is a CLI to aid in upgrading Amazon EKS clusters

use anyhow::Result;
use clap::{CommandFactory, Parser};
use eksup::{Cli, Commands, analyze, create};
use tracing_subscriber::FmtSubscriber;

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();

  let subscriber = FmtSubscriber::builder()
    .with_max_level(cli.verbose.tracing_level_filter())
    .without_time()
    .pretty()
    .finish();
  tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

  match cli.commands {
    Commands::Analyze(args) => analyze(args).await?,
    Commands::Create(args) => create(args).await?,
    Commands::Completion { shell } => {
      let mut cmd = Cli::command();
      clap_complete::generate(shell, &mut cmd, "eksup", &mut std::io::stdout());
    }
    Commands::Man => {
      clap_mangen::Man::new(Cli::command()).render(&mut std::io::stdout())?;
    }
  }

  Ok(())
}
