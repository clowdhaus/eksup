use std::str;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::{output, version};

/// The different types of strategies for upgrading a cluster
///
/// `InPlace`: the control plane is updated in-place by Amazon EKS
/// `BlueGreen`: an entirely new cluster is created alongside the existing
/// and the workloads+traffic will need to be migrated to the new cluster
#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
pub enum Strategy {
  InPlace,
  // BlueGreen,
}

/// The default cluster upgrade strategy is `InPlace`
impl Default for Strategy {
  fn default() -> Self {
    Self::InPlace
  }
}

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
pub enum CreateOptions {
  Analysis,
  Playbook,
}

/// The default cluster upgrade strategy is `InPlace`
impl Default for CreateOptions {
  fn default() -> Self {
    Self::Analysis
  }
}

/// Compute constructs supported by Amazon EKS the data plane
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, ValueEnum)]
pub enum Compute {
  EksManaged,
  SelfManaged,
  FargateProfile,
}

/// Analyze an Amazon EKS cluster prior to upgrading
#[derive(Args, Clone, Debug, Serialize, Deserialize)]
pub struct Analysis {
  /// The name of the cluster to analyze
  #[arg(long, alias = "name", value_enum)]
  pub cluster_name: String,

  /// The AWS region where the cluster is provisioned
  #[arg(long)]
  pub region: Option<String>,

  #[arg(long, alias = "ofmt", value_enum, default_value_t)]
  pub output_format: output::OutputFormat,

  #[arg(long, alias = "otype", value_enum, default_value_t)]
  pub output_type: output::OutputType,

  #[arg(long, alias = "ofile")]
  pub output_filename: Option<String>,
}

/// Create a playbook for upgrading an Amazon EKS cluster
#[derive(Args, Clone, Debug, Serialize, Deserialize)]
pub struct Playbook {
  /// The name of the cluster
  #[arg(long, default_value = "<CLUSTER_NAME>")]
  pub cluster_name: Option<String>,

  /// The cluster's current Kubernetes version
  #[arg(long, value_enum)]
  pub cluster_version: version::KubernetesVersion,

  /// Array of compute types used in the data plane
  #[arg(long, value_enum, num_args = 1..=3)]
  pub compute: Option<Vec<Compute>>,

  /// Whether the AMI used is custom or not (provided by AWS)
  #[arg(long)]
  pub custom_ami: bool,

  /// Name of the output file
  #[arg(short, long, default_value = "playbook.md")]
  pub filename: String,

  /// The cluster upgrade strategy
  #[arg(short, long, value_enum, default_value_t)]
  pub strategy: Strategy,
}

#[derive(Clone, Debug, Subcommand, Serialize, Deserialize)]
pub enum CreateCommands {
  Playbook(Playbook),
}

/// Analyze an Amazon EKS cluster prior to upgrading
#[derive(Args, Clone, Debug, Serialize, Deserialize)]
pub struct Create {
  #[command(subcommand)]
  pub command: CreateCommands,

  /// The name of the cluster to analyze
  #[arg(long, alias = "name", value_enum)]
  pub cluster_name: String,

  /// The AWS region where the cluster is provisioned
  #[arg(long)]
  pub region: Option<String>,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
  Analyze(Analysis),
  Create(Create),
}

#[derive(Parser, Debug)]
#[command(author, about, version)]
#[command(propagate_version = true)]
pub struct Cli {
  #[command(subcommand)]
  pub(crate) commands: Commands,
}
