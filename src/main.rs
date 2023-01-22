#![warn(missing_docs)]

//! `eksup` is a CLI to aid in upgrading Amazon EKS clusters

mod analysis;
mod cli;
mod eks;
mod finding;
mod k8s;
mod output;
mod playbook;
mod version;

use std::process;

use anyhow::*;
use clap::Parser;
use cli::{Cli, Commands, CreateCommands};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
  let cli = Cli::parse();

  match &cli.commands {
    Commands::Analyze(args) => {
      let aws_config = eks::get_config(&args.region.clone()).await?;
      let eks_client = aws_sdk_eks::Client::new(&aws_config);
      let cluster = eks::get_cluster(&eks_client, &args.cluster).await?;

      // All checks and validations on input should happen above/before running the analysis
      let results = analysis::analyze(&aws_config, &cluster).await?;
      output::output(&results, &args.format, &args.output).await?;
    }
    Commands::Create(args) => {
      match &args.command {
        CreateCommands::Playbook(playbook) => {
          // Query Kubernetes first so that we can get AWS details that require them
          let aws_config = eks::get_config(&playbook.region.clone()).await?;
          let eks_client = aws_sdk_eks::Client::new(&aws_config);
          let cluster = eks::get_cluster(&eks_client, &playbook.cluster).await?;
          let cluster_version = cluster.version().unwrap().to_owned();

          if version::LATEST.eq(&cluster_version) {
            println!("Cluster is already at the latest supported version: {cluster_version}");
            println!("Nothing to upgrade at this time");
            return Ok(());
          }

          if let Err(err) = playbook::create(playbook) {
            eprintln!("{err}");
            process::exit(2);
          }
        }
      }
    }
  }

  Ok(())
}
