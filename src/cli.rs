use std::str;

use clap::{Parser, Subcommand, ValueEnum};
use strum_macros::Display;

pub const LATEST: &str = "1.24";

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
    /// The cluster's current Kubernetes version
    #[arg(long, value_enum)]
    pub cluster_version: ClusterVersion,

    /// Whether an EKS managed node group is used
    #[arg(long)]
    pub eks_managed_node_group: bool,

    /// Whether an self-managed node group is used
    #[arg(long)]
    pub self_managed_node_group: bool,

    /// Whether a Fargate Profile is used
    #[arg(long)]
    pub fargate_profile: bool,

    /// Whether the AMI used is custom or not (provided by AWS)
    #[arg(long)]
    pub custom_ami: bool,

    /// Name of the output file
    #[arg(short, long, default_value = "playbook.md")]
    pub filename: String,
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
pub struct Upgrade {
    #[command(subcommand)]
    pub command: Commands,
}
