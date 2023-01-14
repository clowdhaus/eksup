use std::env;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_autoscaling::{
  model::{AutoScalingGroup, Filter as AsgFilter},
  Client as AsgClient,
};
use aws_sdk_ec2::{model::Subnet, Client as Ec2Client};
use aws_sdk_eks::{
  model::{Addon, Cluster, FargateProfile, Nodegroup},
  Client as EksClient,
};
use aws_types::region::Region;
use serde::{Deserialize, Serialize};

pub async fn get_config(region: Option<String>) -> aws_config::SdkConfig {
  // TODO - fix this ugliness
  let region_provider = match region {
    Some(region) => RegionProviderChain::first_try(Region::new(region)).or_default_provider(),
    None => RegionProviderChain::first_try(env::var("AWS_REGION").ok().map(Region::new))
      .or_default_provider(),
  };

  aws_config::from_env().region(region_provider).load().await
}

pub async fn get_cluster(client: &EksClient, name: &str) -> Result<Cluster, anyhow::Error> {
  let req = client.describe_cluster().name(name);
  let resp = req.send().await?;

  // TODO - handle error check here for cluster not found
  let cluster = resp
    .cluster
    .unwrap_or_else(|| panic!("Cluster {name} not found"));

  Ok(cluster)
}

pub async fn get_subnets(
  client: &Ec2Client,
  subnet_ids: Vec<String>,
) -> Result<Vec<Subnet>, anyhow::Error> {
  if subnet_ids.is_empty() {
    return Ok(Vec::new());
  }

  let subnets = client
    .describe_subnets()
    .set_subnet_ids(Some(subnet_ids))
    .send()
    .await?
    .subnets
    .unwrap();

  Ok(subnets)
}

pub async fn get_eks_managed_nodegroups(
  client: &EksClient,
  cluster_name: &str,
) -> Result<Vec<Nodegroup>, anyhow::Error> {
  let nodegroup_names = client
    .list_nodegroups()
    .cluster_name(cluster_name)
    // TODO - paginate this
    .max_results(100)
    .send()
    .await?
    .nodegroups
    .unwrap_or_default();

  let mut nodegroups = Vec::new();

  for nodegroup_name in nodegroup_names {
    let response = client
      .describe_nodegroup()
      .cluster_name(cluster_name)
      .nodegroup_name(nodegroup_name)
      .send()
      .await?
      .nodegroup;

    if let Some(nodegroup) = response {
      nodegroups.push(nodegroup);
    }
  }

  Ok(nodegroups)
}

// TODO - querying on tags will return EKS managed node groups as well
// TODO - We will need to de-dupe
pub async fn get_self_managed_nodegroups(
  client: &AsgClient,
  cluster_name: &str,
) -> Result<Vec<AutoScalingGroup>, anyhow::Error> {
  let keys = vec![
    format!("k8s.io/cluster/{cluster_name}"),
    format!("kubernetes.io/cluster/{cluster_name}"),
  ];

  let filter = AsgFilter::builder()
    .set_name(Some("tag-key".to_string()))
    .set_values(Some(keys))
    .build();

  let response = client
    .describe_auto_scaling_groups()
    .filters(filter)
    .send()
    .await?;

  // Filter out EKS managed node groups by the EKS MNG applied tag
  if let Some(groups) = response.auto_scaling_groups().map(|groups| groups.to_vec()) {
    let filtered = groups
      .into_iter()
      .filter(|group| {
        group
          .tags()
          .unwrap_or_default()
          .iter()
          .all(|tag| tag.key().unwrap_or_default() != "eks:nodegroup-name")
      })
      .collect();

    return Ok(filtered);
  }

  Ok(vec![])
}

pub async fn get_fargate_profiles(
  client: &EksClient,
  cluster_name: &str,
) -> Result<Vec<FargateProfile>, anyhow::Error> {
  let profile_names = client
    .list_fargate_profiles()
    .cluster_name(cluster_name)
    // TODO - paginate this
    .max_results(100)
    .send()
    .await?
    .fargate_profile_names
    .unwrap_or_default();

  let mut profiles = Vec::new();

  for profile_name in &profile_names {
    let response = client
      .describe_fargate_profile()
      .cluster_name(cluster_name)
      .fargate_profile_name(profile_name)
      .send()
      .await?
      .fargate_profile;

    if let Some(profile) = response {
      profiles.push(profile);
    }
  }

  Ok(profiles)
}

pub(crate) async fn get_addons(
  client: &EksClient,
  cluster_name: &str,
) -> Result<Vec<Addon>, anyhow::Error> {
  let addon_names = client
    .list_addons()
    .cluster_name(cluster_name)
    // TODO - paginate this
    .max_results(100)
    .send()
    .await?
    .addons
    .unwrap_or_default();

  let mut addons = Vec::new();

  for addon_name in &addon_names {
    let response = client
      .describe_addon()
      .cluster_name(cluster_name)
      .addon_name(addon_name)
      .send()
      .await?
      .addon;

    if let Some(addon) = response {
      addons.push(addon);
    }
  }

  Ok(addons)
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AddonVersion {
  /// Latest supported version of the addon
  pub(crate) latest: String,
  /// Default version of the addon used by the service
  pub(crate) default: String,
  /// Supported versions for the given Kubernetes version
  /// This maintains the ordering of latest version to oldest
  pub(crate) supported: Vec<String>,
  /// Associated Kubernetes version for compatibility
  pub(crate) kubernetes_version: String,
}

/// Get the addon version details for the given addon and Kubernetes version
///
/// Returns associated version details for a given addon that, primarily used
/// for version compatibility checks and/or updgrade recommendations
pub(crate) async fn get_addon_versions(
  client: &EksClient,
  name: &str,
  kubernetes_version: &str,
) -> Result<AddonVersion, anyhow::Error> {
  // Get all of the addon versions supported for the given addon and Kubernetes version
  let describe = client
    .describe_addon_versions()
    .addon_name(name)
    .kubernetes_version(kubernetes_version)
    .send()
    .await?;

  // Since we are providing an addon name, we are only concerned with the first and only item
  let addon = describe.addons().unwrap().get(0).unwrap();
  let latest_version_info = addon.addon_versions.as_ref().unwrap().get(0).unwrap();
  let latest_version = latest_version_info.addon_version.as_ref().unwrap();
  // The default version as specified by the EKS API for a given addon and Kubenrnetes version
  let default_version = addon
    .addon_versions
    .as_ref()
    .unwrap()
    .iter()
    .filter(|v| {
      v.compatibilities
        .as_ref()
        .unwrap()
        .iter()
        .any(|c| c.default_version)
    })
    .map(|v| v.addon_version.as_ref().unwrap())
    .next()
    .unwrap();

  // Get the list of ALL supported version for this addon and Kubernetes version
  // The results maintain the oder of latest version to oldest
  let supported: Vec<String> = addon
    .addon_versions
    .as_ref()
    .unwrap()
    .iter()
    .map(|v| v.addon_version.as_ref().unwrap().to_owned())
    .collect();

  Ok(AddonVersion {
    latest: latest_version.to_owned(),
    default: default_version.to_owned(),
    supported,
    kubernetes_version: kubernetes_version.to_owned(),
  })
}
