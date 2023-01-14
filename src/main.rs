mod analysis;
mod aws;
mod cli;
mod k8s;
mod output;
mod playbook;
mod version;

use std::process;

use anyhow::*;
use clap::Parser;
use cli::{Cli, Commands};

pub const LATEST: &str = "1.24";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
  let cli = Cli::parse();

  match &cli.command {
    Commands::CreatePlaybook(args) => {
      let cluster_version = args.cluster_version.to_string();
      if LATEST.eq(&cluster_version) {
        println!("Cluster is already at the latest supported version: {cluster_version}");
        println!("Nothing to upgrade at this time");
        return Ok(());
      }

      if let Err(err) = playbook::create(args) {
        eprintln!("{err}");
        process::exit(2);
      }
    }

    Commands::Analyze(args) => {
      // Query Kubernetes first so that we can get AWS details that require them
      let aws_config = aws::get_config(args.region.clone()).await;
      let eks_client = aws_sdk_eks::Client::new(&aws_config);
      let cluster = aws::get_cluster(&eks_client, &args.cluster_name).await?;

      let results = analysis::execute(&aws_config, &cluster).await?;
      let filename = match &args.output_type {
        output::OutputType::File => args
          .output_filename
          .as_ref()
          .expect("--output-file is required when --output-type is `file`"),
        _ => "",
      };

      output::output(&results, &args.output_format, &args.output_type, filename).await?;
    }
  }

  Ok(())
}
