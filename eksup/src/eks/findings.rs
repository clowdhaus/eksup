use anyhow::Result;
use aws_sdk_autoscaling::Client as AsgClient;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_eks::{Client as EksClient, types::Cluster};
use kube::Client as K8sClient;
use serde::{Deserialize, Serialize};

use crate::eks::{checks, resources};

/// Findings related to the cluster itself, primarily the control plane
#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterFindings {
  /// The health of the cluster as reported by the Amazon EKS API
  pub cluster_health: Vec<checks::ClusterHealthIssue>,
}

/// Collects the cluster findings from the Amazon EKS API
pub async fn get_cluster_findings(cluster: &Cluster) -> Result<ClusterFindings> {
  let cluster_health = checks::cluster_health(cluster)?;

  Ok(ClusterFindings { cluster_health })
}

/// Networking/subnet findings, primarily focused on IP exhaustion/number of available IPs
#[derive(Debug, Serialize, Deserialize)]
pub struct SubnetFindings {
  /// The Amazon EKS service requires at least 5 available IPs in order to upgrade a cluster in-place
  pub control_plane_ips: Vec<checks::InsufficientSubnetIps>,
  /// This is the number of IPs available to pods when custom networking is enabled on the AWS VPC CNI,
  /// pulling the available number of IPs for the subnets listed in the ENIConfig resource(s)
  pub pod_ips: Vec<checks::InsufficientSubnetIps>,
}

/// Collects findings related to networking and subnets
///
/// TBD - currently this checks if there are at least 5 available IPs for the control plane cross account ENIs
/// and provides feedback on IPs available for pods when utilizing custom networking. However, it does not cover
/// the IPs for the nodes or nodes and pods when custom networking is not involved. Should these IPs be reported
/// as a whole (treat the data plane as a whole, reporting how many IPs are available), reported per compute
/// construct (each MNG, ASG, Fargate profile takes n-number of subnets, should these groupings be reported
/// individually since it will affect that construct but not necessarily the entire data plane), or a combination
/// of those two?
pub async fn get_subnet_findings(
  ec2_client: &Ec2Client,
  k8s_client: &K8sClient,
  cluster: &Cluster,
) -> Result<SubnetFindings> {
  let control_plane_ips = checks::control_plane_ips(ec2_client, cluster).await?;
  // TODO - The required and recommended number of IPs need to be configurable to allow users who have better
  // TODO - context on their environment as to what should be required and recommended
  let pod_ips = checks::pod_ips(ec2_client, k8s_client, 16, 256).await?;

  Ok(SubnetFindings {
    control_plane_ips,
    pod_ips,
  })
}

/// Findings related to the EKS addons
///
/// Either native EKS addons or addons deployed through the AWS Marketplace integration.
/// It does NOT include custom addons or services deployed by users using kubectl/Helm/etc.,
/// it is only evaluating those that can be accessed via the AWS EKS API
#[derive(Debug, Serialize, Deserialize)]
pub struct AddonFindings {
  /// Determines whether or not the current addon version is supported by Amazon EKS in the
  /// intended upgrade target Kubernetes version
  pub version_compatibility: Vec<checks::AddonVersionCompatibility>,
  /// Reports any health issues as reported by the Amazon EKS addon API
  pub health: Vec<checks::AddonHealthIssue>,
}

/// Collects the addon findings from the Amazon EKS addon API
pub async fn get_addon_findings(
  eks_client: &EksClient,
  cluster_name: &str,
  cluster_version: &str,
  target_minor: i32,
) -> Result<AddonFindings> {
  let addons = resources::get_addons(eks_client, cluster_name).await?;

  let version_compatibility = checks::addon_version_compatibility(eks_client, cluster_version, target_minor, &addons).await?;
  let health = checks::addon_health(&addons)?;

  Ok(AddonFindings {
    version_compatibility,
    health,
  })
}

/// Findings related to the data plane infrastructure components
///
/// This does not include findings for resources that are running on the cluster, within the data plane
/// (pods, deployments, etc.)
#[derive(Debug, Serialize, Deserialize)]
pub struct DataPlaneFindings {
  /// The health of the EKS managed node groups as reported by the Amazon EKS managed node group API
  pub eks_managed_nodegroup_health: Vec<checks::NodegroupHealthIssue>,
  /// Will show if the current launch template provided to the Amazon EKS managed node group is NOT the latest
  /// version since this may potentially introduce additional changes that were not planned for just the upgrade
  /// (i.e. - any changes that may have been introduced in the launch template versions that have not been deployed)
  pub eks_managed_nodegroup_update: Vec<checks::ManagedNodeGroupUpdate>,
  /// Similar to the `eks_managed_nodegroup_update` except for self-managed node groups (autoscaling groups)
  pub self_managed_nodegroup_update: Vec<checks::AutoscalingGroupUpdate>,
  /// EKS managed nodegroups using deprecated AL2 AMI types
  pub al2_ami_deprecation: Vec<checks::Al2AmiDeprecation>,

  /// The names of the EKS managed node groups
  pub eks_managed_nodegroups: Vec<String>,
  /// The names of the self-managed node groups (autoscaling groups)
  pub self_managed_nodegroups: Vec<String>,
  /// The names of the Fargate profiles
  pub fargate_profiles: Vec<String>,
}

/// Collects the data plane findings
pub async fn get_data_plane_findings(
  asg_client: &AsgClient,
  ec2_client: &Ec2Client,
  eks_client: &EksClient,
  cluster: &Cluster,
  target_minor: i32,
) -> Result<DataPlaneFindings> {
  let cluster_name = cluster.name().unwrap_or_default();

  let eks_mngs = resources::get_eks_managed_nodegroups(eks_client, cluster_name).await?;
  let self_mngs = resources::get_self_managed_nodegroups(asg_client, cluster_name).await?;
  let fargate_profiles = resources::get_fargate_profiles(eks_client, cluster_name).await?;

  let eks_managed_nodegroup_health = checks::eks_managed_nodegroup_health(&eks_mngs)?;
  let al2_ami_deprecation = checks::al2_ami_deprecation(&eks_mngs, target_minor)?;
  let mut eks_managed_nodegroup_update = Vec::new();
  for eks_mng in &eks_mngs {
    let update = checks::eks_managed_nodegroup_update(ec2_client, eks_mng).await?;
    eks_managed_nodegroup_update.push(update);
  }

  let mut self_managed_nodegroup_update = Vec::new();
  for self_mng in &self_mngs {
    let update = checks::self_managed_nodegroup_update(ec2_client, self_mng).await?;
    self_managed_nodegroup_update.push(update);
  }

  Ok(DataPlaneFindings {
    eks_managed_nodegroup_health,
    eks_managed_nodegroup_update: eks_managed_nodegroup_update
      .into_iter()
      .flatten()
      .collect::<Vec<checks::ManagedNodeGroupUpdate>>(),
    self_managed_nodegroup_update: self_managed_nodegroup_update
      .into_iter()
      .flatten()
      .collect::<Vec<checks::AutoscalingGroupUpdate>>(),
    al2_ami_deprecation,
    // Pass through to avoid additional API calls
    eks_managed_nodegroups: eks_mngs
      .iter()
      .map(|mng| mng.nodegroup_name().unwrap_or_default().to_owned())
      .collect(),
    self_managed_nodegroups: self_mngs
      .iter()
      .map(|asg| asg.auto_scaling_group_name().unwrap_or_default().to_owned())
      .collect(),
    fargate_profiles: fargate_profiles
      .iter()
      .map(|fp| fp.fargate_profile_name().unwrap_or_default().to_owned())
      .collect(),
  })
}
