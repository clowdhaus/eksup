use std::str;

use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::output;

#[derive(Parser, Debug)]
#[command(author, about, version)]
#[command(propagate_version = true)]
pub struct Cli {
  #[command(subcommand)]
  pub(crate) commands: Commands,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
  #[command(arg_required_else_help = true)]
  Analyze(Analysis),
  #[command(arg_required_else_help = true)]
  Create(Create),
}

/// Analyze an Amazon EKS cluster for potential upgrade issues
#[derive(Args, Clone, Debug, Serialize, Deserialize)]
pub struct Analysis {
  /// The name of the cluster to analyze
  #[arg(short, long, alias = "cluster-name", value_enum)]
  pub cluster: String,

  /// The AWS region where the cluster is provisioned
  #[arg(short, long)]
  pub region: Option<String>,

  #[arg(short, long, value_enum, default_value_t)]
  pub format: output::OutputFormat,

  /// Write to file instead of stdout
  #[arg(short, long)]
  pub output: Option<String>,

  /// Exclude recommendations from the output
  #[arg(long)]
  pub ignore_recommended: bool
}

/// Create artifacts using the analysis data
#[derive(Args, Clone, Debug, Serialize, Deserialize)]
pub struct Create {
  #[command(subcommand)]
  pub command: CreateCommands,
}

#[derive(Clone, Debug, Subcommand, Serialize, Deserialize)]
pub enum CreateCommands {
  #[command(arg_required_else_help = true)]
  Playbook(Playbook),
}

/// Create a playbook for upgrading an Amazon EKS cluster
#[derive(Args, Clone, Debug, Serialize, Deserialize)]
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
  pub ignore_recommended: bool
}
