use std::str;

use clap::{Parser, Subcommand, ValueEnum};
use strum_macros::Display;

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq)]
pub enum ClusterVersion {
    #[strum(serialize = "1.20")]
    V20,
    #[strum(serialize = "1.21")]
    V21,
    #[strum(serialize = "1.22")]
    V22,
    #[strum(serialize = "1.23")]
    V23,
    #[strum(serialize = "1.24")]
    V24,
    // #[strum(serialize = "1.25")]
    // V25,
    // #[strum(serialize = "1.26")]
    // V26,
    // #[strum(serialize = "1.27")]
    // V27,
}

impl ValueEnum for ClusterVersion {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::V20,
            Self::V21,
            Self::V22,
            Self::V23,
            Self::V24,
            // Self::V25,
            // Self::V26,
            // Self::V27,
        ]
    }

    fn to_possible_value<'a>(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Self::V20 => Some(clap::builder::PossibleValue::new("1.20")),
            Self::V21 => Some(clap::builder::PossibleValue::new("1.21")),
            Self::V22 => Some(clap::builder::PossibleValue::new("1.22")),
            Self::V23 => Some(clap::builder::PossibleValue::new("1.23")),
            Self::V24 => Some(clap::builder::PossibleValue::new("1.24")),
            // Self::V25 => Some(clap::builder::PossibleValue::new("1.25")),
            // Self::V26 => Some(clap::builder::PossibleValue::new("1.26")),
            // Self::V27 => Some(clap::builder::PossibleValue::new("1.27")),
        }
    }
}

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq)]
pub enum Compute {
    EksManaged,
    SelfManaged,
    FargateProfile,
}

impl ValueEnum for Compute {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::EksManaged, Self::SelfManaged, Self::FargateProfile]
    }

    fn to_possible_value<'a>(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Self::EksManaged => Some(clap::builder::PossibleValue::new("eks")),
            Self::SelfManaged => Some(clap::builder::PossibleValue::new("self")),
            Self::FargateProfile => Some(clap::builder::PossibleValue::new("fargate")),
        }
    }
}

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, ValueEnum)]
pub enum Strategy {
    InPlace,
    // BlueGreen,
}

impl Default for Strategy {
    fn default() -> Self {
        Self::InPlace
    }
}

/// Analyze an Amazon EKS cluster prior to upgrading
#[derive(Parser, Debug, Clone)]
pub struct Analysis {
    /// The name of the cluster to analyze
    #[arg(long, value_enum)]
    pub cluster_name: String,
}

/// Create a playbook for upgrading an Amazon EKS cluster
#[derive(Parser, Debug, Clone)]
pub struct Playbook {
    /// The name of the cluster
    #[arg(long, default_value = "<CLUSTER_NAME>")]
    pub cluster_name: Option<String>,

    /// The cluster's current Kubernetes version
    #[arg(long, value_enum)]
    pub cluster_version: ClusterVersion,

    /// Array of compute types used in the data plane
    #[arg(long, value_enum, num_args = 1..=3)]
    pub compute: Vec<Compute>,

    /// Whether the AMI used is custom or not (provided by AWS)
    #[arg(long)]
    pub custom_ami: bool,

    /// Name of the output file
    #[arg(short, long, default_value = "playbook.md")]
    pub filename: String,

    /// The cluster upgrade strategy
    #[arg(short, long, value_enum, default_value_t)]
    pub strategy: Strategy,
    // /// Render output to stdout
    // #[arg(long)]
    // pub stdout: bool,

    // /// The cluster hosts stateful workloads
    // #[arg(long)]
    // pub stateful: bool,

    // /// The cluster hosts multi-tenant teams
    // #[arg(long)]
    // pub multi_tenant: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Analyze(Analysis),
    CreatePlaybook(Playbook),
}

#[derive(Parser, Debug)]
#[command(author, about, version)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
