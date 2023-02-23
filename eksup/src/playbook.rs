use std::{collections::HashMap, fs};

use anyhow::Result;
use aws_sdk_eks::model::Cluster;
use handlebars::Handlebars;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

use crate::{analysis, eks, finding::Findings, version, Playbook};

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
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateData {
  region: String,
  cluster_name: String,
  current_version: String,
  target_version: String,
  k8s_release_url: String,
  k8s_deprecation_url: String,
  control_plane_ips: String,
  pod_ips: String,
  cluster_health: String,
  addon_health: String,
  addon_version_compatibility: String,
  data_plane_findings: eks::DataPlaneFindings,
  version_skew: String,
  eks_managed_nodegroup_template: String,
  self_managed_nodegroup_template: String,
  fargate_profile_template: String,
  // kubernetes_findings: k8s::KubernetesFindings,
  min_replicas: String,
  min_ready_seconds: String,
  pod_topology_distribution: String,
  readiness_probe: String,
  termination_grace_period: String,
  docker_socket: String,
  pod_security_policy: String,
}

fn get_release_data() -> Result<HashMap<Version, Release>> {
  let data_file = Templates::get("data.yaml").unwrap();
  let contents = std::str::from_utf8(data_file.data.as_ref())?;
  let data: HashMap<Version, Release> = serde_yaml::from_str(contents)?;

  Ok(data)
}

#[derive(Debug, Serialize, Deserialize)]
struct EksManagedNodeGroupTemplateData {
  region: String,
  cluster_name: String,
  target_version: String,
  eks_managed_nodegroup_health: String,
  eks_managed_nodegroup_update: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SelfManagedNodeGroupTemplateData {
  region: String,
  cluster_name: String,
  target_version: String,
  self_managed_nodegroup_update: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FargateProfileTemplateData {
  region: String,
  cluster_name: String,
  target_version: String,
}

fn char_replace(text: String) -> String {
  text
    .replace("&#x60;", "`")
    .replace("&#x27;", "'")
    .replace("&lt;", "<")
    .replace("&amp;lt;", "<")
    .replace("&gt;", ">")
    .replace("&amp;gt;", ">")
    .replace("&quot;", "\"")
    .replace("&#x3D;", "=")
}

pub(crate) fn create(args: &Playbook, cluster: &Cluster, analysis: analysis::Results) -> Result<()> {
  let mut handlebars = Handlebars::new();
  handlebars.register_embed_templates::<Templates>()?;

  let region = args.region.as_ref().unwrap().to_owned();
  let cluster_name = cluster.name().unwrap();
  let cluster_version = cluster.version().unwrap();
  let target_version = version::get_target_version(cluster_version)?;
  let default_playbook_name = format!("{cluster_name}_v{target_version}_upgrade.md");

  let release_data = get_release_data()?;
  let release = release_data.get(&target_version).unwrap();

  let cluster_findings = analysis.cluster;
  let data_plane_findings = analysis.data_plane;
  let subnet_findings = analysis.subnets;
  let addon_findings = analysis.addons;
  let kubernetes_findings = analysis.kubernetes;

  // Render sub-templates for data plane components
  let eks_mng_tmpl_data = EksManagedNodeGroupTemplateData {
    region: region.to_owned(),
    cluster_name: cluster_name.to_owned(),
    target_version: target_version.to_owned(),
    eks_managed_nodegroup_health: data_plane_findings
      .eks_managed_nodegroup_health
      .to_markdown_table("\t")?,
    eks_managed_nodegroup_update: data_plane_findings
      .eks_managed_nodegroup_update
      .to_markdown_table("\t")?,
  };
  let eks_managed_nodegroup_template = char_replace(handlebars.render("eks-managed-nodegroup.md", &eks_mng_tmpl_data)?);

  let self_mng_tmpl_data = SelfManagedNodeGroupTemplateData {
    region: region.to_owned(),
    cluster_name: cluster_name.to_owned(),
    target_version: target_version.to_owned(),
    self_managed_nodegroup_update: data_plane_findings
      .self_managed_nodegroup_update
      .to_markdown_table("\t")?,
  };
  let self_managed_nodegroup_template =
    char_replace(handlebars.render("self-managed-nodegroup.md", &self_mng_tmpl_data)?);

  let fargate_tmpl_data = FargateProfileTemplateData {
    region: region.to_owned(),
    cluster_name: cluster_name.to_owned(),
    target_version: target_version.to_owned(),
  };
  let fargate_profile_template = char_replace(handlebars.render("fargate-node.md", &fargate_tmpl_data)?);

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
    control_plane_ips: subnet_findings.control_plane_ips.to_markdown_table("\t")?,
    pod_ips: subnet_findings.pod_ips.to_markdown_table("\t")?,
    cluster_health: cluster_findings.cluster_health.to_markdown_table("\t")?,
    addon_health: addon_findings.health.to_markdown_table("\t")?,
    addon_version_compatibility: addon_findings.version_compatibility.to_markdown_table("\t")?,
    version_skew: data_plane_findings.version_skew.to_markdown_table("\t")?,
    data_plane_findings,
    eks_managed_nodegroup_template,
    self_managed_nodegroup_template,
    fargate_profile_template,
    // kubernetes_findings,
    min_replicas: kubernetes_findings.min_replicas.to_markdown_table("\t")?,
    min_ready_seconds: kubernetes_findings.min_ready_seconds.to_markdown_table("\t")?,
    pod_topology_distribution: kubernetes_findings.pod_topology_distribution.to_markdown_table("\t")?,
    readiness_probe: kubernetes_findings.readiness_probe.to_markdown_table("\t")?,
    termination_grace_period: kubernetes_findings.termination_grace_period.to_markdown_table("\t")?,
    docker_socket: kubernetes_findings.docker_socket.to_markdown_table("\t")?,
    pod_security_policy: kubernetes_findings.pod_security_policy.to_markdown_table("\t")?,
  };

  let filename = match &args.filename {
    Some(filename) => filename,
    None => &default_playbook_name,
  };

  // TODO = handlebars should be able to handle backticks and apostrophes
  // Need to figure out why this isn't the case currently
  // let mut output_file = File::create("playbook.md")?;
  let rendered = handlebars.render("playbook.md", &tmpl_data)?;
  // handlebars.render_to_write("playbook.tmpl", &data, &mut output_file)?;
  let replaced = char_replace(rendered);
  fs::write(filename, replaced)?;

  Ok(())
}
