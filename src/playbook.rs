use std::{collections::HashMap, fs};

use aws_sdk_eks::model::Cluster;
use handlebars::Handlebars;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

use crate::{analysis, cli::Playbook, finding::Findings, version};

/// Embeds the contents of the `templates/` directory into the binary
///
/// This struct contains both the templates used for rendering the playbook
/// as well as the static data used for populating the playbook templates
/// embedded into the binary for distribution
#[derive(RustEmbed)]
#[folder = "templates/"]
struct Templates;

/// Relevant data for a Kubernetes release
///
/// Used to populate the playbook templates with the data associated
/// to a specific Kubernetes release version
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Release {
  release_url: String,
  deprecation_url: Option<String>,
}

/// Type alias for Kubernetes version string (i.e. - "1.21")
type Version = String;

/// Data to populate the template(s) for rendering the upgrade playbook
///
/// This combines the static data from the `data.yaml` embedded along with
/// data collected from CLI arguments provided by users and is used to
/// populate the playbook templates when rendered. This also serves as
/// the central authority for the data/inputs used to populate the playbook
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemplateData {
  ///
  region: String,
  cluster_name: String,
  current_version: String,
  target_version: String,
  k8s_release_url: String,
  k8s_deprecation_url: String,
  version_skew: Option<String>,
  control_plane_ips: Option<String>,
  cluster_health: Option<String>,
  addon_health: Option<String>,
  addon_version_compatibility: Option<String>,
  eks_managed_nodegroups: Vec<String>,
  eks_managed_nodegroup_template: String,
  self_managed_nodegroups: Vec<String>,
  self_managed_nodegroup_template: String,
}

fn get_release_data() -> Result<HashMap<Version, Release>, anyhow::Error> {
  let data_file = Templates::get("data.yaml").unwrap();
  let contents = std::str::from_utf8(data_file.data.as_ref())?;
  let data: HashMap<Version, Release> = serde_yaml::from_str(contents)?;

  Ok(data)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EksManagedNodeGroupTemplateData {
  region: String,
  cluster_name: String,
  target_version: String,
  eks_managed_nodegroup_health: Option<String>,
  eks_managed_nodegroup_update: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SelfManagedNodeGroupTemplateData {
  region: String,
  cluster_name: String,
  target_version: String,
  self_managed_nodegroup_update: Option<String>,
}

pub(crate) fn create(args: &Playbook, cluster: &Cluster, analysis: analysis::Results) -> Result<(), anyhow::Error> {
  let mut handlebars = Handlebars::new();
  handlebars.register_embed_templates::<Templates>()?;

  let region = args.region.as_ref().unwrap().to_owned();
  let cluster_name = cluster.name().unwrap();
  let cluster_version = cluster.version().unwrap();
  let target_version = version::get_target_version(cluster_version)?;

  let release_data = get_release_data()?;
  let release = release_data.get(cluster_version).unwrap();

  let cluster_findings = analysis.cluster;
  let data_plane_findings = analysis.data_plane;
  let subnet_findings = analysis.subnets;
  let addon_findings = analysis.addons;

  // Render sub-templates for data plane components
  let eks_managed_nodegroup_health = data_plane_findings.eks_managed_nodegroup_health.to_markdown_table("\t");
  let eks_managed_nodegroup_update = data_plane_findings.eks_managed_nodegroup_update.to_markdown_table("\t");
  let eks_mng_tmpl_data = EksManagedNodeGroupTemplateData {
    region: region.to_owned(),
    cluster_name: cluster_name.to_owned(),
    target_version: target_version.to_owned(),
    eks_managed_nodegroup_health,
    eks_managed_nodegroup_update,
  };
  let eks_managed_nodegroup_template = handlebars.render("eks-managed-nodegroup.md", &eks_mng_tmpl_data)?;

  let self_managed_nodegroup_update = data_plane_findings
    .self_managed_nodegroup_update
    .to_markdown_table("\t");
  let self_mng_tmpl_data = SelfManagedNodeGroupTemplateData {
    region: region.to_owned(),
    cluster_name: cluster_name.to_owned(),
    target_version: target_version.to_owned(),
    self_managed_nodegroup_update,
  };
  let self_managed_nodegroup_template = handlebars.render("self-managed-nodegroup.md", &self_mng_tmpl_data)?;

  let tmpl_data = TemplateData {
    region,
    cluster_name: cluster_name.to_owned(),
    current_version: cluster_version.to_owned(),
    target_version,
    k8s_release_url: release.release_url.to_string(),
    k8s_deprecation_url: match &release.deprecation_url {
      Some(url) => url.to_string(),
      None => "".to_string(),
    },
    version_skew: data_plane_findings.version_skew.to_markdown_table("\t"),
    control_plane_ips: subnet_findings.control_plane_ips.to_markdown_table("\t"),
    cluster_health: cluster_findings.cluster_health.to_markdown_table("\t"),
    addon_health: addon_findings.health.to_markdown_table("\t"),
    addon_version_compatibility: addon_findings.version_compatibility.to_markdown_table("\t"),
    eks_managed_nodegroups: data_plane_findings.eks_managed_nodegroups,
    eks_managed_nodegroup_template,
    self_managed_nodegroups: data_plane_findings.self_managed_nodegroups,
    self_managed_nodegroup_template,
  };

  let filename = match &args.filename {
    Some(filename) => filename,
    // TODO - update default name to include cluster name, versions, etc. that would make it unique
    None => "playbook.md",
  };

  // TODO = handlebars should be able to handle backticks and apostrophes
  // Need to figure out why this isn't the case currently
  // let mut output_file = File::create("playbook.md")?;
  let rendered = handlebars.render("playbook.md", &tmpl_data)?;
  // handlebars.render_to_write("playbook.tmpl", &data, &mut output_file)?;
  let replaced = rendered
    .replace("&#x60;", "`")
    .replace("&#x27;", "'")
    .replace("&lt;", "<")
    .replace("&amp;lt;", "<")
    .replace("&gt;", ">")
    .replace("&amp;gt;", ">")
    .replace("&quot;", "\"")
    .replace("&#x3D;", "=");
  fs::write(filename, replaced)?;

  Ok(())
}
