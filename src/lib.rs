use std::fs;
use std::str;

use anyhow::*;
use clap::{Parser, ValueEnum};
use handlebars::{to_json, Handlebars};
use rust_embed::RustEmbed;
use serde_json::value::{Map, Value as Json};
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

impl ClusterVersion {
    pub fn hyphenated_version(&self) -> String {
        match self {
            ClusterVersion::V19 => "1-19",
            ClusterVersion::V20 => "1-20",
            ClusterVersion::V21 => "1-21",
            ClusterVersion::V22 => "1-22",
            ClusterVersion::V23 => "1-23",
            ClusterVersion::V24 => "1-24",
        }
        .to_string()
    }
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

#[derive(RustEmbed)]
#[folder = "templates/"]
struct Templates;

fn get_kubernetes_deprecations(version: ClusterVersion) -> Result<String, anyhow::Error> {
    let url = "https://kubernetes.io/docs/reference/using-api/deprecation-guide/#v";
    let hyphenated_version = version.hyphenated_version();

    let deprecations = match version {
        ClusterVersion::V22 => format!("{url}{hyphenated_version}"),
        _ => "".to_string(),
    };

    Ok(deprecations)
}

pub fn render_template_data(upgrade: Upgrade) -> Map<String, Json> {
    let mut data = Map::new();

    // Parse out minor version string into an integer
    let current_minor_version = upgrade
        .cluster_version
        .to_string()
        .split('.')
        .collect::<Vec<&str>>()[1]
        .parse::<i32>()
        .unwrap();

    let target_version = format!("1.{}", current_minor_version + 1);
    let kubernetes_deprecations = get_kubernetes_deprecations(upgrade.cluster_version).unwrap();

    data.insert(
        "current_version".to_string(),
        to_json(upgrade.cluster_version.to_string()),
    );
    data.insert("target_version".to_string(), to_json(target_version));
    data.insert(
        "kubernetes_deprecations".to_string(),
        to_json(kubernetes_deprecations),
    );

    let hyphenated_version = upgrade.cluster_version.hyphenated_version();
    let eks_version =
        Templates::get(format!("eks/versions/{hyphenated_version}.md").as_str()).unwrap();
    let contents = str::from_utf8(eks_version.data.as_ref())
        .unwrap()
        .to_string();

    data.insert("eks_version".to_string(), to_json(contents));
    data
}

pub fn render(upgrade: Upgrade) -> Result<(), anyhow::Error> {
    let mut handlebars = Handlebars::new();
    handlebars.register_embed_templates::<Templates>().unwrap();

    let data = render_template_data(upgrade);

    // TODO = handlebars should be able to handle backticks and apostrophes
    // Need to figure out why this isn't the case currently
    // let mut output_file = File::create("playbook.md")?;
    let rendered = handlebars.render("playbook.tmpl", &data)?;
    // handlebars.render_to_write("playbook.tmpl", &data, &mut output_file)?;

    let replaced = rendered.replace("&#x60;", "`").replace("&#x27;", "'");
    fs::write("playbook.md", replaced)?;

    Ok(())
}
