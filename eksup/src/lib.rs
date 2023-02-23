mod analysis;
mod eks;
mod finding;
mod k8s;
mod output;
mod playbook;
mod version;

use std::{process, str};

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(author, about, version)]
#[command(propagate_version = true)]
pub struct Cli {
  #[command(subcommand)]
  pub commands: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
  #[command(arg_required_else_help = true)]
  Analyze(Analysis),
  #[command(arg_required_else_help = true)]
  Create(Create),
}

/// Analyze an Amazon EKS cluster for potential upgrade issues
#[derive(Args, Debug, Serialize, Deserialize)]
pub struct Analysis {
  /// The name of the cluster to analyze
  #[arg(short, long, alias = "cluster-name", value_enum)]
  pub cluster: String,

  /// The AWS region where the cluster is provisioned
  #[arg(short, long)]
  pub region: Option<String>,

  #[arg(short, long, value_enum, default_value_t)]
  pub format: output::Format,

  /// Write to file instead of stdout
  #[arg(short, long)]
  pub output: Option<String>,

  /// Exclude recommendations from the output
  #[arg(long)]
  pub ignore_recommended: bool,
}

/// Create artifacts using the analysis data
#[derive(Args, Debug, Serialize, Deserialize)]
pub struct Create {
  #[command(subcommand)]
  pub command: CreateCommands,
}

#[derive(Debug, Subcommand, Serialize, Deserialize)]
pub enum CreateCommands {
  #[command(arg_required_else_help = true)]
  Playbook(Playbook),
}

/// Create a playbook for upgrading an Amazon EKS cluster
#[derive(Args, Debug, Serialize, Deserialize)]
pub struct Playbook {
  /// The name of the cluster to analyze
  #[arg(short, long, alias = "cluster-name", value_enum)]
  pub cluster: String,

  /// The AWS region where the cluster is provisioned
  #[arg(short, long)]
  pub region: Option<String>,

  /// Name of the playbook saved locally
  #[arg(short, long)]
  pub filename: Option<String>,

  /// Exclude recommendations from the output
  #[arg(long)]
  pub ignore_recommended: bool,
}

/// Someting TODO
pub async fn analyze(args: &Analysis) -> Result<()> {
  let aws_config = eks::get_config(&args.region.to_owned()).await?;
  let eks_client = aws_sdk_eks::Client::new(&aws_config);
  let cluster = eks::get_cluster(&eks_client, &args.cluster).await?;

  // All checks and validations on input should happen above/before running the analysis
  let results = analysis::analyze(&aws_config, &cluster).await?;
  output::output(&results, &args.format, &args.output).await?;

  Ok(())
}

/// Someting TODO
pub async fn create(args: &Create) -> Result<()> {
  match &args.command {
    CreateCommands::Playbook(playbook) => {
      // Query Kubernetes first so that we can get AWS details that require them
      let aws_config = eks::get_config(&playbook.region.to_owned()).await?;
      let eks_client = aws_sdk_eks::Client::new(&aws_config);
      let cluster = eks::get_cluster(&eks_client, &playbook.cluster).await?;
      let cluster_version = cluster.version().unwrap();

      if version::LATEST.eq(cluster_version) {
        println!("Cluster is already at the latest supported version: {cluster_version}");
        println!("Nothing to upgrade at this time");
        return Ok(());
      }

      let results = analysis::analyze(&aws_config, &cluster).await?;

      if let Err(err) = playbook::create(playbook, &cluster, results) {
        eprintln!("{err}");
        process::exit(2);
      }
    }
  }

  Ok(())
}