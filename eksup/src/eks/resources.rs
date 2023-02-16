use std::{collections::HashSet, env};

use anyhow::{bail, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_autoscaling::{
  model::{AutoScalingGroup, Filter as AsgFilter},
  Client as AsgClient,
};
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_eks::{
  model::{Addon, Cluster, FargateProfile, Nodegroup},
  Client as EksClient,
};
use aws_types::region::Region;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

/// Get the configuration to authn/authz with AWS that will be used across AWS clients
pub async fn get_config(region: &Option<String>) -> Result<aws_config::SdkConfig> {
  let aws_region = match region {
    Some(region) => Region::new(region.to_owned()),
    None => env::var("AWS_REGION").ok().map(Region::new).unwrap(),
  };

  let region_provider = RegionProviderChain::first_try(aws_region).or_default_provider();

  Ok(aws_config::from_env().region(region_provider).load().await)
}

/// Describe the cluster to get its full details
pub async fn get_cluster(client: &EksClient, name: &str) -> Result<Cluster> {
  let request = client.describe_cluster().name(name);
  let response = request.send().await?;

  match response.cluster {
    Some(cluster) => Ok(cluster),
    None => bail!("Cluster {name} not found"),
  }
}

/// Container for the subnet IDs and their total available IPs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SubnetIPs {
  pub(crate) ids: Vec<String>,
  pub(crate) available_ips: i32,
}

/// Describe the subnets provided by ID
///
/// This will show the number of available IPs for evaluating
/// IP contention/exhaustion across the various subnets in use
/// by the control plane ENIs, the nodes, and the pods (when custom
/// networking is enabled)
pub(crate) async fn get_subnet_ips(client: &Ec2Client, subnet_ids: Vec<String>) -> Result<SubnetIPs> {
  let subnets = client
    .describe_subnets()
    .set_subnet_ids(Some(subnet_ids))
    .send()
    .await?
    .subnets
    .unwrap();

  let available_ips = subnets
    .iter()
    .map(|subnet| subnet.available_ip_address_count.unwrap())
    .sum();

  let ids = subnets
    .iter()
    .map(|subnet| subnet.subnet_id().unwrap().to_string())
    .collect::<Vec<String>>();

  Ok(SubnetIPs { ids, available_ips })
}

pub async fn get_addons(client: &EksClient, cluster_name: &str) -> Result<Vec<Addon>> {
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

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct AddonVersion {
  /// Latest supported version of the addon
  pub latest: String,
  /// Default version of the addon used by the service
  pub default: String,
  /// Supported versions for the given Kubernetes version
  /// This maintains the ordering of latest version to oldest
  #[tabled(skip)]
  pub supported_versions: HashSet<String>,
}

/// Get the addon version details for the given addon and Kubernetes version
///
/// Returns associated version details for a given addon that, primarily used
/// for version compatibility checks and/or upgrade recommendations
pub(crate) async fn get_addon_versions(
  client: &EksClient,
  name: &str,
  kubernetes_version: &str,
) -> Result<AddonVersion> {
  // Get all of the addon versions supported for the given addon and Kubernetes version
  let describe = client
    .describe_addon_versions()
    .addon_name(name)
    .kubernetes_version(kubernetes_version)
    .send()
    .await?;

  // Since we are providing an addon name, we are only concerned with the first and only item
  let addon = describe.addons().unwrap().get(0).unwrap();
  let addon_version = addon.addon_versions().unwrap();
  let latest_version = addon_version.first().unwrap().addon_version().unwrap();

  // The default version as specified by the EKS API for a given addon and Kubernetes version
  let default_version = addon
    .addon_versions()
    .unwrap()
    .iter()
    .filter(|v| v.compatibilities().unwrap().iter().any(|c| c.default_version))
    .map(|v| v.addon_version().unwrap())
    .next()
    .unwrap();

  // Get the list of ALL supported version for this addon and Kubernetes version
  // The results maintain the oder of latest version to oldest
  let supported_versions: HashSet<String> = addon
    .addon_versions()
    .unwrap()
    .iter()
    .map(|v| v.addon_version().unwrap().to_owned())
    .collect();

  Ok(AddonVersion {
    latest: latest_version.to_owned(),
    default: default_version.to_owned(),
    supported_versions,
  })
}

pub async fn get_eks_managed_nodegroups(client: &EksClient, cluster_name: &str) -> Result<Vec<Nodegroup>> {
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

pub async fn get_self_managed_nodegroups(client: &AsgClient, cluster_name: &str) -> Result<Vec<AutoScalingGroup>> {
  let keys = vec![
    format!("k8s.io/cluster/{cluster_name}"),
    format!("kubernetes.io/cluster/{cluster_name}"),
  ];

  let filter = AsgFilter::builder()
    .set_name(Some("tag-key".to_string()))
    .set_values(Some(keys))
    .build();

  let response = client.describe_auto_scaling_groups().filters(filter).send().await?;
  let groups = response.auto_scaling_groups().map(|groups| groups.to_vec());

  // Filter out EKS managed node groups by the EKS MNG applied tag
  match groups {
    Some(groups) => {
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

      Ok(filtered)
    }
    None => Ok(vec![]),
  }
}

pub async fn get_fargate_profiles(client: &EksClient, cluster_name: &str) -> Result<Vec<FargateProfile>> {
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

#[derive(Clone, Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct LaunchTemplate {
  /// Name of the launch template
  #[tabled(skip)]
  pub name: String,
  /// The ID of the launch template
  #[tabled(rename = "LAUNCH TEMP ID")]
  pub id: String,
  /// The version of the launch template currently used/specified in the autoscaling group
  #[tabled(rename = "CURRENT")]
  pub current_version: String,
  /// The latest version of the launch template
  #[tabled(rename = "LATEST")]
  pub latest_version: String,
}

pub(crate) async fn get_launch_template(client: &Ec2Client, id: &str) -> Result<LaunchTemplate> {
  let output = client
    .describe_launch_templates()
    .set_launch_template_ids(Some(vec![id.to_string()]))
    .send()
    .await?;

  let template = output
    .launch_templates
    .unwrap()
    .into_iter()
    .map(|lt| LaunchTemplate {
      name: lt.launch_template_name.unwrap(),
      id: lt.launch_template_id.unwrap(),
      current_version: lt.default_version_number.unwrap().to_string(),
      latest_version: lt.latest_version_number.unwrap().to_string(),
    })
    .next();

  match template {
    Some(t) => Ok(t),
    None => bail!("Unable to find launch template with id: {id}"),
  }
}
