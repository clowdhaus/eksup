use anyhow::{Context, Result};
use aws_sdk_eks::types::Cluster;
use serde::{Deserialize, Serialize};

use crate::{clients::{AwsClients, K8sClients}, eks, finding::Findings, k8s, version};

/// Container of all findings collected
#[derive(Debug, Serialize, Deserialize)]
pub struct Results {
  pub cluster: eks::ClusterFindings,
  pub subnets: eks::SubnetFindings,
  pub data_plane: eks::DataPlaneFindings,
  pub addons: eks::AddonFindings,
  pub kubernetes: k8s::KubernetesFindings,
}

impl Results {
  /// Remove all findings where remediation is `Recommended`, keeping only `Required`
  pub fn filter_recommended(&mut self) {
    self.cluster.cluster_health.retain(|f| !f.finding.remediation.is_recommended());
    self.subnets.control_plane_ips.retain(|f| !f.finding.remediation.is_recommended());
    self.subnets.pod_ips.retain(|f| !f.finding.remediation.is_recommended());
    self.addons.health.retain(|f| !f.finding.remediation.is_recommended());
    self.addons.version_compatibility.retain(|f| !f.finding.remediation.is_recommended());
    self.data_plane.eks_managed_nodegroup_health.retain(|f| !f.finding.remediation.is_recommended());
    self.data_plane.eks_managed_nodegroup_update.retain(|f| !f.finding.remediation.is_recommended());
    self.data_plane.self_managed_nodegroup_update.retain(|f| !f.finding.remediation.is_recommended());
    self.data_plane.al2_ami_deprecation.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.version_skew.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.min_replicas.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.min_ready_seconds.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.pod_topology_distribution.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.readiness_probe.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.termination_grace_period.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.docker_socket.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.kube_proxy_version_skew.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.kube_proxy_ipvs_mode.retain(|f| !f.finding.remediation.is_recommended());
    self.kubernetes.ingress_nginx_retirement.retain(|f| !f.finding.remediation.is_recommended());
  }

  /// Renders all findings as a formatted stdout table string
  pub fn to_stdout_table(&self) -> Result<String> {
    let mut output = String::new();

    output.push_str(&self.subnets.pod_ips.to_stdout_table()?);
    output.push_str(&self.subnets.control_plane_ips.to_stdout_table()?);
    output.push_str(&self.cluster.cluster_health.to_stdout_table()?);
    output.push_str(&self.data_plane.eks_managed_nodegroup_health.to_stdout_table()?);
    output.push_str(&self.addons.health.to_stdout_table()?);
    output.push_str(&self.addons.version_compatibility.to_stdout_table()?);
    output.push_str(&self.data_plane.eks_managed_nodegroup_update.to_stdout_table()?);
    output.push_str(&self.data_plane.self_managed_nodegroup_update.to_stdout_table()?);
    output.push_str(&self.data_plane.al2_ami_deprecation.to_stdout_table()?);
    output.push_str(&self.kubernetes.version_skew.to_stdout_table()?);
    output.push_str(&self.kubernetes.min_replicas.to_stdout_table()?);
    output.push_str(&self.kubernetes.min_ready_seconds.to_stdout_table()?);
    output.push_str(&self.kubernetes.pod_topology_distribution.to_stdout_table()?);
    output.push_str(&self.kubernetes.readiness_probe.to_stdout_table()?);
    output.push_str(&self.kubernetes.termination_grace_period.to_stdout_table()?);
    output.push_str(&self.kubernetes.docker_socket.to_stdout_table()?);
    output.push_str(&self.kubernetes.kube_proxy_version_skew.to_stdout_table()?);
    output.push_str(&self.kubernetes.kube_proxy_ipvs_mode.to_stdout_table()?);
    output.push_str(&self.kubernetes.ingress_nginx_retirement.to_stdout_table()?);

    Ok(output)
  }
}

/// Analyze the cluster provided to collect all reported findings
pub async fn analyze(
  aws: &impl AwsClients,
  k8s: &impl K8sClients,
  cluster: &Cluster,
  target_minor: i32,
) -> Result<Results> {
  let cluster_name = cluster.name().context("Cluster name missing from API response")?;
  let cluster_version = cluster.version().context("Cluster version missing from API response")?;
  let control_plane_minor = version::parse_minor(cluster_version)?;

  let cluster_findings = eks::get_cluster_findings(cluster)?;

  let (subnet_findings, addon_findings, dataplane_findings, kubernetes_findings) = tokio::try_join!(
    eks::get_subnet_findings(aws, k8s, cluster),
    eks::get_addon_findings(aws, cluster_name, cluster_version, target_minor),
    eks::get_data_plane_findings(aws, cluster, target_minor),
    k8s::get_kubernetes_findings(k8s, control_plane_minor, target_minor),
  )?;

  Ok(Results {
    cluster: cluster_findings,
    subnets: subnet_findings,
    addons: addon_findings,
    data_plane: dataplane_findings,
    kubernetes: kubernetes_findings,
  })
}
