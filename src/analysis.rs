use aws_sdk_autoscaling::Client as AsgClient;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_eks::{model::Cluster, Client as EksClient};
use kube::Client as K8sClient;
use serde::{Deserialize, Serialize};

use crate::{eks, finding, k8s};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ClusterFindings {
  pub(crate) cluster_health: Vec<finding::Code>,
}

async fn get_cluster_findings(cluster: &Cluster) -> Result<ClusterFindings, anyhow::Error> {
  let cluster_health = eks::cluster_health(cluster).await?;

  Ok(ClusterFindings { cluster_health })
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SubnetFindings {
  pub(crate) control_plane_ips: Option<finding::Code>,
  pub(crate) pod_ips: Option<finding::Code>,
}

async fn get_subnet_findings(
  ec2_client: &Ec2Client,
  k8s_client: &K8sClient,
  cluster: &Cluster,
) -> Result<SubnetFindings, anyhow::Error> {
  let control_plane_ips = eks::control_plane_ips(ec2_client, cluster).await?;
  let pod_ips = eks::pod_ips(ec2_client, k8s_client, 16, 256).await?;

  Ok(SubnetFindings {
    control_plane_ips,
    pod_ips,
  })
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AddonFindings {
  pub(crate) version_compatibility: Vec<finding::Code>,
  pub(crate) health: Vec<finding::Code>,
}

async fn get_addon_findings(
  eks_client: &EksClient,
  cluster_name: &str,
  cluster_version: &str,
) -> Result<AddonFindings, anyhow::Error> {
  let addons = eks::get_addons(eks_client, cluster_name).await?;

  let version_compatibility = eks::addon_version_compatibility(eks_client, cluster_version, &addons).await?;
  let health = eks::addon_health(&addons).await?;

  Ok(AddonFindings {
    version_compatibility,
    health,
  })
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct DataPlaneFindings {
  pub(crate) version_skew: Vec<finding::Code>,
  pub(crate) eks_managed_node_group_health: Vec<finding::Code>,
  pub(crate) eks_managed_node_group_update: Vec<finding::Code>,
  pub(crate) self_managed_node_group_update: Vec<finding::Code>,
}

async fn get_data_plane_findings(
  asg_client: &AsgClient,
  ec2_client: &Ec2Client,
  eks_client: &EksClient,
  k8s_client: &kube::Client,
  cluster: &Cluster,
) -> Result<DataPlaneFindings, anyhow::Error> {
  let cluster_name = cluster.name.as_ref().unwrap();
  let cluster_version = cluster.version.as_ref().unwrap();

  let eks_mngs = eks::get_eks_managed_nodegroups(eks_client, cluster_name).await?;
  let self_mngs = eks::get_self_managed_nodegroups(asg_client, cluster_name).await?;

  let version_skew = k8s::version_skew(k8s_client, cluster_version).await?;
  let eks_managed_node_group_health = eks::eks_managed_node_group_health(&eks_mngs).await?;
  let mut eks_managed_node_group_update = Vec::new();
  for eks_mng in &eks_mngs {
    let update = eks::eks_managed_node_group_update(ec2_client, eks_mng).await?;
    eks_managed_node_group_update.push(update);
  }
  let mut self_managed_node_group_update = Vec::new();
  for self_mng in &self_mngs {
    let update = eks::self_managed_node_group_update(ec2_client, self_mng).await?;
    self_managed_node_group_update.push(update);
  }

  Ok(DataPlaneFindings {
    version_skew,
    eks_managed_node_group_health,
    eks_managed_node_group_update: eks_managed_node_group_update.into_iter().flatten().collect(),
    self_managed_node_group_update: self_managed_node_group_update.into_iter().flatten().collect(),
  })
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Findings {
  pub(crate) cluster: ClusterFindings,
  pub(crate) subnets: SubnetFindings,
  pub(crate) data_plane: DataPlaneFindings,
  pub(crate) addons: AddonFindings,
}

pub(crate) async fn analyze(
  aws_shared_config: &aws_config::SdkConfig,
  cluster: &Cluster,
) -> Result<Findings, anyhow::Error> {
  // Construct clients once
  let asg_client = aws_sdk_autoscaling::Client::new(aws_shared_config);
  let ec2_client = aws_sdk_ec2::Client::new(aws_shared_config);
  let eks_client = aws_sdk_eks::Client::new(aws_shared_config);
  let k8s_client = kube::Client::try_default().await?;

  let cluster_name = cluster.name.as_ref().unwrap();
  let cluster_version = cluster.version.as_ref().unwrap();

  let cluster_findings = get_cluster_findings(cluster).await?;
  let subnet_findings = get_subnet_findings(&ec2_client, &k8s_client, cluster).await?;
  let addon_findings = get_addon_findings(&eks_client, cluster_name, cluster_version).await?;
  let dataplane_findings = get_data_plane_findings(&asg_client, &ec2_client, &eks_client, &k8s_client, cluster).await?;

  Ok(Findings {
    cluster: cluster_findings,
    subnets: subnet_findings,
    addons: addon_findings,
    data_plane: dataplane_findings,
  })
}
