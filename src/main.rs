mod analysis;
mod aws;
mod cli;
mod k8s;
mod output;
mod playbook;
mod version;

// use std::process;

use anyhow::*;
use clap::Parser;
use cli::{Cli, Commands, CreateOptions};

pub const LATEST: &str = "1.24";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
  let cli = Cli::parse();

  match &cli.commands {
    Commands::Create(args) => {
      // Query Kubernetes first so that we can get AWS details that require them
      let aws_config = aws::get_config(args.region.clone()).await;
      let eks_client = aws_sdk_eks::Client::new(&aws_config);
      let cluster = aws::get_cluster(&eks_client, &args.cluster_name).await?;
      let cluster_version = cluster.version.as_ref().unwrap().to_owned();

      match args.option {
        CreateOptions::Analysis => {
          let filename = match &args.output_type {
            output::OutputType::File => args
              .output_filename
              .as_ref()
              .expect("--output-file is required when --output-type is `file`"),
            _ => "",
          };

          // All checks and validations on input should happen above/before running the analysis
          let results = analysis::execute(&aws_config, &cluster).await?;

          output::output(&results, &args.output_format, &args.output_type, filename).await?;
        }

        CreateOptions::Playbook => {
          if LATEST.eq(&cluster_version) {
            println!("Cluster is already at the latest supported version: {cluster_version}");
            println!("Nothing to upgrade at this time");
            return Ok(());
          }

          // if let Err(err) = playbook::create(&args) {
          //   eprintln!("{err}");
          //   process::exit(2);
          // }
        }
      }
    }
  }

  Ok(())
}
