use anyhow::{Context, Result, bail};
use aws_sdk_eks::types::Cluster;
use serde::{Deserialize, Serialize};

use crate::{eks, finding::Findings, k8s, version};

/// Container of all findings collected
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Results {
  pub(crate) cluster: eks::ClusterFindings,
  pub(crate) subnets: eks::SubnetFindings,
  pub(crate) data_plane: eks::DataPlaneFindings,
  pub(crate) addons: eks::AddonFindings,
  pub(crate) kubernetes: k8s::KubernetesFindings,
}

impl Results {
  /// Remove all findings where remediation is `Recommended`, keeping only `Required`
  pub(crate) fn filter_recommended(&mut self) {
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
  pub(crate) fn to_stdout_table(&self) -> Result<String> {
    let mut output = String::new();

    // Ordered sub-group (AWS -> EKS -> K8s) and check number
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
pub(crate) async fn analyze(aws_shared_config: &aws_config::SdkConfig, cluster: &Cluster) -> Result<Results> {
  // Construct clients once
  let asg_client = aws_sdk_autoscaling::Client::new(aws_shared_config);
  let ec2_client = aws_sdk_ec2::Client::new(aws_shared_config);
  let eks_client = aws_sdk_eks::Client::new(aws_shared_config);

  let cluster_name = cluster.name().context("Cluster name missing from API response")?;

  let k8s_client = match kube::Client::try_default().await {
    Ok(client) => client,
    Err(_) => {
      bail!(
        "Unable to connect to cluster. Ensure kubeconfig file is present and updated to connect to the cluster.
      Try: aws eks update-kubeconfig --name {cluster_name}"
      );
    }
  };

  let cluster_version = cluster.version().context("Cluster version missing from API response")?;
  let target_minor = version::get_target_version(cluster_version)?;

  let cluster_findings = eks::get_cluster_findings(cluster).await?;
  let subnet_findings = eks::get_subnet_findings(&ec2_client, &k8s_client, cluster).await?;
  let addon_findings = eks::get_addon_findings(&eks_client, cluster_name, cluster_version).await?;
  let dataplane_findings = eks::get_data_plane_findings(&asg_client, &ec2_client, &eks_client, cluster, target_minor).await?;
  let kubernetes_findings = k8s::get_kubernetes_findings(&k8s_client, cluster_version, target_minor).await?;

  Ok(Results {
    cluster: cluster_findings,
    subnets: subnet_findings,
    addons: addon_findings,
    data_plane: dataplane_findings,
    kubernetes: kubernetes_findings,
  })
}
