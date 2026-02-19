use anyhow::{Context, Result};
use aws_sdk_autoscaling::Client as AsgClient;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_eks::{Client as EksClient, types::Cluster};
use kube::Client as K8sClient;
use serde::{Deserialize, Serialize};

use crate::eks::{checks, resources};
use crate::version;

/// Findings related to the cluster itself, primarily the control plane
#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterFindings {
  /// The health of the cluster as reported by the Amazon EKS API
  pub cluster_health: Vec<checks::ClusterHealthIssue>,
}

/// Collects the cluster findings from the Amazon EKS API
pub fn get_cluster_findings(cluster: &Cluster) -> Result<ClusterFindings> {
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
pub async fn get_subnet_findings(
  ec2_client: &Ec2Client,
  k8s_client: &K8sClient,
  cluster: &Cluster,
) -> Result<SubnetFindings> {
  // Fetch control plane subnet IPs
  let control_plane_subnet_ids = match cluster.resources_vpc_config() {
    Some(vpc_config) => vpc_config.subnet_ids().to_owned(),
    None => vec![],
  };
  let control_plane_subnet_ips = if control_plane_subnet_ids.is_empty() {
    vec![]
  } else {
    resources::get_subnet_ips(ec2_client, control_plane_subnet_ids).await?
  };

  // Fetch pod subnet IPs (custom networking via ENIConfig)
  let eniconfigs = crate::k8s::get_eniconfigs(k8s_client).await?;
  let pod_subnet_ids: Vec<String> = eniconfigs
    .iter()
    .filter_map(|eniconfig| eniconfig.spec.subnet.clone())
    .collect();
  let pod_subnet_ips = if pod_subnet_ids.is_empty() {
    vec![]
  } else {
    resources::get_subnet_ips(ec2_client, pod_subnet_ids).await?
  };

  // Run pure checks
  let control_plane_ips = checks::control_plane_ips(&control_plane_subnet_ips);
  let pod_ips = checks::pod_ips(&pod_subnet_ips, 16, 256);

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
  let target_k8s_version = version::format_version(target_minor);

  // Pre-fetch all addon version data
  let mut current_versions = std::collections::HashMap::new();
  let mut target_versions = std::collections::HashMap::new();
  for addon in &addons {
    let name = addon.addon_name().unwrap_or_default().to_owned();
    let current = resources::get_addon_versions(eks_client, &name, cluster_version).await?;
    let target = resources::get_addon_versions(eks_client, &name, &target_k8s_version).await?;
    current_versions.insert(name.clone(), current);
    target_versions.insert(name, target);
  }

  let version_compatibility = checks::addon_version_compatibility(&addons, &current_versions, &target_versions);
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

  // Pre-fetch launch templates for EKS managed nodegroups, then run pure check
  let mut eks_managed_nodegroup_update = Vec::new();
  for eks_mng in &eks_mngs {
    let lt = match eks_mng.launch_template() {
      Some(lt_spec) => {
        let lt_id = lt_spec.id().context("Launch template spec missing ID")?;
        Some(resources::get_launch_template(ec2_client, lt_id).await?)
      }
      None => None,
    };
    eks_managed_nodegroup_update.extend(checks::eks_managed_nodegroup_update(eks_mng, lt.as_ref()));
  }

  // Pre-fetch launch templates for self-managed nodegroups, then run pure check
  let mut self_managed_nodegroup_update = Vec::new();
  for self_mng in &self_mngs {
    let lt_spec = self_mng
      .launch_template()
      .context("Launch template not found, launch configuration is not supported")?;
    let lt = resources::get_launch_template(ec2_client, lt_spec.launch_template_id().unwrap_or_default()).await?;
    if let Some(update) = checks::self_managed_nodegroup_update(self_mng, &lt) {
      self_managed_nodegroup_update.push(update);
    }
  }

  Ok(DataPlaneFindings {
    eks_managed_nodegroup_health,
    eks_managed_nodegroup_update,
    self_managed_nodegroup_update,
    al2_ami_deprecation,
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
