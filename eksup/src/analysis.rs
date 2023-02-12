use anyhow::Result;
use aws_sdk_eks::model::Cluster;
use serde::{Deserialize, Serialize};

use crate::{eks, k8s};

/// Container of all findings collected
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Results {
  pub(crate) cluster: eks::ClusterFindings,
  pub(crate) subnets: eks::SubnetFindings,
  pub(crate) data_plane: eks::DataPlaneFindings,
  pub(crate) addons: eks::AddonFindings,
  pub(crate) kubernetes: k8s::KubernetesFindings,
}

/// Analyze the cluster provided to collect all reported findings
pub(crate) async fn analyze(aws_shared_config: &aws_config::SdkConfig, cluster: &Cluster) -> Result<Results> {
  // Construct clients once
  let asg_client = aws_sdk_autoscaling::Client::new(aws_shared_config);
  let ec2_client = aws_sdk_ec2::Client::new(aws_shared_config);
  let eks_client = aws_sdk_eks::Client::new(aws_shared_config);
  let k8s_client = kube::Client::try_default().await?;

  let cluster_name = cluster.name().unwrap();
  let cluster_version = cluster.version().unwrap();

  let cluster_findings = eks::get_cluster_findings(cluster).await?;
  let subnet_findings = eks::get_subnet_findings(&ec2_client, &k8s_client, cluster).await?;
  let addon_findings = eks::get_addon_findings(&eks_client, cluster_name, cluster_version).await?;
  let dataplane_findings =
    eks::get_data_plane_findings(&asg_client, &ec2_client, &eks_client, &k8s_client, cluster).await?;
  let kubernetes_findings = k8s::get_kubernetes_findings(&k8s_client).await?;

  Ok(Results {
    cluster: cluster_findings,
    subnets: subnet_findings,
    addons: addon_findings,
    data_plane: dataplane_findings,
    kubernetes: kubernetes_findings,
  })
}
