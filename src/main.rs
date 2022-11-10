use anyhow::*;
use clap::{Parser, ValueEnum};
use strum_macros::Display;

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq)]
enum ClusterVersion {
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
enum Strategy {
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
struct Upgrade {
    /// The cluster's current Kubernetes version
    #[arg(short, long, value_enum)]
    cluster_version: ClusterVersion,

    /// The cluster upgrade strategy
    #[arg(short, long, value_enum, default_value_t)]
    strategy: Strategy,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Upgrade::parse();

    println!("Hello {:#?}", args);
    println!("v{}", ClusterVersion::V19);

    Ok(())
}
