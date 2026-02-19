use std::{collections::HashMap, fs};

use anyhow::{Context, Result};
use aws_sdk_eks::types::Cluster;
use handlebars::Handlebars;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

use crate::{Playbook, analysis, eks, finding::Findings, version};

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
  min_replicas: String,
  min_ready_seconds: String,
  pod_topology_distribution: String,
  readiness_probe: String,
  termination_grace_period: String,
  docker_socket: String,
  kube_proxy_version_skew: String,
  kube_proxy_ipvs_mode: String,
  ingress_nginx_retirement: String,
  pod_disruption_budgets: String,
}

fn get_release_data() -> Result<HashMap<Version, Release>> {
  let data_file = Templates::get("data.yaml").context("Embedded data.yaml template not found")?;
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
  al2_ami_deprecation: String,
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

/// Render the upgrade playbook markdown from analysis results without writing to disk
pub fn render(region: &str, cluster: &Cluster, analysis: analysis::Results, target_minor: i32) -> Result<String> {
  let mut handlebars = Handlebars::new();
  handlebars.register_escape_fn(handlebars::no_escape);
  handlebars.register_embed_templates::<Templates>()?;

  let cluster_name = cluster.name().context("Cluster name missing")?;
  let cluster_version = cluster.version().context("Cluster version missing")?;
  let target_version = version::format_version(target_minor);

  let release_data = get_release_data()?;
  let release = release_data.get(&target_version)
    .context(format!("No release data found for version {target_version}"))?;

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
    al2_ami_deprecation: data_plane_findings
      .al2_ami_deprecation
      .to_markdown_table("\t")?,
  };
  let eks_managed_nodegroup_template = handlebars.render("eks-managed-nodegroup.md", &eks_mng_tmpl_data)?;

  let self_mng_tmpl_data = SelfManagedNodeGroupTemplateData {
    region: region.to_owned(),
    cluster_name: cluster_name.to_owned(),
    target_version: target_version.to_owned(),
    self_managed_nodegroup_update: data_plane_findings
      .self_managed_nodegroup_update
      .to_markdown_table("\t")?,
  };
  let self_managed_nodegroup_template =
    handlebars.render("self-managed-nodegroup.md", &self_mng_tmpl_data)?;

  let fargate_tmpl_data = FargateProfileTemplateData {
    region: region.to_owned(),
    cluster_name: cluster_name.to_owned(),
    target_version: target_version.to_owned(),
  };
  let fargate_profile_template = handlebars.render("fargate-node.md", &fargate_tmpl_data)?;

  let tmpl_data = TemplateData {
    region: region.to_owned(),
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
    data_plane_findings,
    eks_managed_nodegroup_template,
    self_managed_nodegroup_template,
    fargate_profile_template,
    version_skew: kubernetes_findings.version_skew.to_markdown_table("\t")?,
    min_replicas: kubernetes_findings.min_replicas.to_markdown_table("\t")?,
    min_ready_seconds: kubernetes_findings.min_ready_seconds.to_markdown_table("\t")?,
    pod_topology_distribution: kubernetes_findings.pod_topology_distribution.to_markdown_table("\t")?,
    readiness_probe: kubernetes_findings.readiness_probe.to_markdown_table("\t")?,
    termination_grace_period: kubernetes_findings.termination_grace_period.to_markdown_table("\t")?,
    docker_socket: kubernetes_findings.docker_socket.to_markdown_table("\t")?,
    kube_proxy_version_skew: kubernetes_findings.kube_proxy_version_skew.to_markdown_table("\t")?,
    kube_proxy_ipvs_mode: kubernetes_findings.kube_proxy_ipvs_mode.to_markdown_table("\t")?,
    ingress_nginx_retirement: kubernetes_findings.ingress_nginx_retirement.to_markdown_table("\t")?,
    pod_disruption_budgets: kubernetes_findings.pod_disruption_budgets.to_markdown_table("\t")?,
  };

  let rendered = handlebars.render("playbook.md", &tmpl_data)?;
  Ok(rendered)
}

pub(crate) fn create(args: Playbook, region: String, cluster: &Cluster, analysis: analysis::Results, target_minor: i32) -> Result<()> {
  let cluster_name = cluster.name().context("Cluster name missing")?;
  let target_version = version::format_version(target_minor);
  let default_playbook_name = format!("{cluster_name}_v{target_version}_upgrade.md");

  let rendered = render(&region, cluster, analysis, target_minor)?;

  let filename = args.filename.as_deref().unwrap_or(&default_playbook_name);
  fs::write(filename, rendered)?;
  Ok(())
}
