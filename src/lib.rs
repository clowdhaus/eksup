use anyhow::*;
use clap::{Parser, ValueEnum};
use strum_macros::Display;

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq)]
pub enum ClusterVersion {
    #[strum(serialize = "1.19")]
    V19,
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
}

impl ValueEnum for ClusterVersion {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::V19,
            Self::V20,
            Self::V21,
            Self::V22,
            Self::V23,
            Self::V24,
        ]
    }

    fn to_possible_value<'a>(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Self::V19 => Some(clap::builder::PossibleValue::new("1.19")),
            Self::V20 => Some(clap::builder::PossibleValue::new("1.20")),
            Self::V21 => Some(clap::builder::PossibleValue::new("1.21")),
            Self::V22 => Some(clap::builder::PossibleValue::new("1.22")),
            Self::V23 => Some(clap::builder::PossibleValue::new("1.23")),
            Self::V24 => Some(clap::builder::PossibleValue::new("1.24")),
        }
    }
}

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, ValueEnum)]
pub enum Strategy {
    InPlace,
    BlueGreen,
}

impl Default for Strategy {
    fn default() -> Self {
        Self::InPlace
    }
}

#[derive(Parser, Debug)]
#[command(author, about, version)]
pub struct Upgrade {
    /// The cluster's current Kubernetes version
    #[arg(short, long, value_enum)]
    pub cluster_version: ClusterVersion,

    /// The cluster upgrade strategy
    #[arg(short, long, value_enum, default_value_t)]
    pub strategy: Strategy,

    /// Render output to stdout
    #[arg(long)]
    pub stdout: bool,

    /// The cluster hosts stateful workloads
    #[arg(long)]
    pub stateful: bool,

    /// The cluster hosts multi-tenant teams
    #[arg(long)]
    pub multi_tenant: bool,
}

fn _get_kubernetes_deprecations(version: ClusterVersion) -> Result<String> {
    let url = "https://kubernetes.io/docs/reference/using-api/deprecation-guide/#v";
    let formatted_version = version.to_string().replace('.', "-");

    let deprecations = match version {
        ClusterVersion::V22 => format!("{url}{formatted_version}"),
        _ => "".to_string(),
    };

    Ok(deprecations)
}
