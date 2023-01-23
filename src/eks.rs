use std::{collections::HashSet, env};

use anyhow::bail;
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
use kube::Client as K8sClient;
use serde::{Deserialize, Serialize};

use crate::{
  finding::{self, Findings},
  k8s, version,
};

/// Get the configuration to authn/authz with AWS that will be used across AWS clients
pub(crate) async fn get_config(region: &Option<String>) -> Result<aws_config::SdkConfig, anyhow::Error> {
  let aws_region = match region {
    Some(region) => Region::new(region.to_owned()),
    None => env::var("AWS_REGION").ok().map(Region::new).unwrap(),
  };

  let region_provider = RegionProviderChain::first_try(aws_region).or_default_provider();

  Ok(aws_config::from_env().region(region_provider).load().await)
}

/// Describe the cluster to get its full details
pub(crate) async fn get_cluster(client: &EksClient, name: &str) -> Result<Cluster, anyhow::Error> {
  let req = client.describe_cluster().name(name);
  let resp = req.send().await?;

  // TODO - handle error check here for cluster not found
  let cluster = resp.cluster.unwrap_or_else(|| panic!("Cluster {name} not found"));

  Ok(cluster)
}

/// Cluster health issue data
///
/// Nearly identical to the SDK's `ClusterIssue` but allows us to serialize/deserialize
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ClusterHealthIssue {
  pub(crate) code: String,
  pub(crate) message: String,
  pub(crate) resource_ids: Vec<String>,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

impl Findings for Vec<ClusterHealthIssue> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}:white_check_mark: - There are no reported health issues on the cluster control plane"
      ));
    }

    let mut table = String::new();
    table.push_str(&format!(
      "{leading_whitespace}|   _   | Code  | Message | Resource IDs |\n"
    ));
    table.push_str(&format!(
      "{leading_whitespace}| :---: | :---: | :------ | :----------- |\n"
    ));

    for finding in self {
      table.push_str(&format!(
        "{}| {} | {} | {} | {} |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.code,
        finding.message,
        finding.resource_ids.join(", ")
      ))
    }

    Some(table)
  }
}

/// Check for any reported health issues on the cluster control plane
pub(crate) async fn cluster_health(cluster: &Cluster) -> Result<Vec<ClusterHealthIssue>, anyhow::Error> {
  let health = cluster.health();

  match health {
    Some(health) => {
      let issues = health
        .issues()
        .unwrap()
        .to_owned()
        .iter()
        .map(|issue| {
          let code = &issue.code().unwrap().to_owned();

          ClusterHealthIssue {
            code: code.as_str().to_string(),
            message: issue.message().unwrap().to_string(),
            resource_ids: issue.resource_ids().unwrap().to_owned(),
            remediation: finding::Remediation::Required,
            fcode: finding::Code::EKS002,
          }
        })
        .collect();

      Ok(issues)
    }
    None => Ok(vec![]),
  }
}

/// Container for the subnet IDs and their total available IPs
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct SubnetIPs {
  ids: Vec<String>,
  available_ips: i32,
}

/// Describe the subnets provided by ID
///
/// This will show the number of available IPs for evaluating
/// IP contention/exhaustion across the various subnets in use
/// by the control plane ENIs, the nodes, and the pods (when custom
/// networking is enabled)
async fn get_subnet_ips(client: &Ec2Client, subnet_ids: Vec<String>) -> Result<SubnetIPs, anyhow::Error> {
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

/// Subnet details that can affect upgrade behavior
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct InsufficientSubnetIps {
  pub(crate) ids: Vec<String>,
  pub(crate) available_ips: i32,
  pub(crate) remediation: finding::Remediation,
  pub(crate) code: finding::Code,
}

impl Findings for Option<InsufficientSubnetIps> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    match self {
      Some(finding) => {
      let mut table = String::new();
      table.push_str(&format!("{leading_whitespace}|   -   | Subnet IDs  | Available IPs |\n"));
      table.push_str(&format!("{leading_whitespace}| :---: | :---------- | :-----------: |\n"));
      table.push_str(&format!(
        "{}| {} | {} | {} |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.ids.iter().map(|f| format!("`{f}`")).collect::<Vec<String>>().join(", "),
        finding.available_ips,
      ));

      Some(table)
      },
      None => {
        Some(format!("{leading_whitespace}:white_check_mark: - There is sufficient IP space in the subnets used by the control plane"))
      }
    }
  }
}

pub(crate) async fn control_plane_ips(
  ec2_client: &Ec2Client,
  cluster: &Cluster,
) -> Result<Option<InsufficientSubnetIps>, anyhow::Error> {
  let subnet_ids = cluster.resources_vpc_config().unwrap().subnet_ids().unwrap().to_owned();

  let subnet_ips = get_subnet_ips(ec2_client, subnet_ids).await?;
  if subnet_ips.available_ips >= 5 {
    return Ok(None);
  }

  let finding = InsufficientSubnetIps {
    ids: subnet_ips.ids,
    available_ips: subnet_ips.available_ips,
    remediation: finding::Remediation::Required,
    code: finding::Code::EKS001,
  };

  Ok(Some(finding))
}

/// Check if the subnets used by the pods will support an upgrade
///
/// This checks for the `ENIConfig` custom resource that is used to configure
/// the AWS VPC CNI for custom networking. The subnet listed for each ENIConfig
/// is queried for its relevant data used to report on the available IPs
pub(crate) async fn pod_ips(
  ec2_client: &Ec2Client,
  k8s_client: &K8sClient,
  required_ips: i32,
  recommended_ips: i32,
) -> Result<Option<InsufficientSubnetIps>, anyhow::Error> {
  let eniconfigs = k8s::get_eniconfigs(k8s_client).await?;
  if eniconfigs.is_empty() {
    return Ok(None);
  }

  let subnet_ids = eniconfigs
    .iter()
    .map(|eniconfig| eniconfig.spec.subnet.as_ref().unwrap().to_owned())
    .collect();

  let subnet_ips = get_subnet_ips(ec2_client, subnet_ids).await?;

  if subnet_ips.available_ips >= recommended_ips {
    return Ok(None);
  }

  let remediation = if subnet_ips.available_ips >= required_ips {
    finding::Remediation::Required
  } else {
    finding::Remediation::Recommended
  };
  let finding = InsufficientSubnetIps {
    ids: subnet_ips.ids,
    available_ips: subnet_ips.available_ips,
    remediation,
    code: finding::Code::AWS002,
  };

  Ok(Some(finding))
}

pub(crate) async fn get_addons(client: &EksClient, cluster_name: &str) -> Result<Vec<Addon>, anyhow::Error> {
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AddonVersion {
  /// Latest supported version of the addon
  pub(crate) latest: String,
  /// Default version of the addon used by the service
  pub(crate) default: String,
  /// Supported versions for the given Kubernetes version
  /// This maintains the ordering of latest version to oldest
  pub(crate) supported_versions: HashSet<String>,
}

/// Get the addon version details for the given addon and Kubernetes version
///
/// Returns associated version details for a given addon that, primarily used
/// for version compatibility checks and/or upgrade recommendations
async fn get_addon_versions(
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
  let latest_version_info = addon.addon_versions().unwrap().get(0).unwrap();
  let latest_version = latest_version_info.addon_version().unwrap();
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

/// Details of the addon as viewed from an upgrade perspective
///
/// Contains the associated version information to compare the current version
/// of the addon relative to the current "desired" version, as well as
/// relative to the target Kubernetes version "desired" version. It
/// also contains any potential health issues as reported by the EKS API.
/// The intended goal is to be able to plot a path of what steps a user either
/// needs to take to upgrade the cluster, or should consider taking in terms
/// of a recommendation to update to the latest supported version.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AddonVersionCompatibility {
  pub(crate) name: String,
  /// The current version of the add-on
  pub(crate) version: String,
  /// The default and latest add-on versions for the current Kubernetes version
  pub(crate) current_kubernetes_version: AddonVersion,
  /// The default and latest add-on versions for the target Kubernetes version
  pub(crate) target_kubernetes_version: AddonVersion,
  pub(crate) remediation: finding::Remediation,
  pub(crate) code: finding::Code,
}

impl Findings for Vec<AddonVersionCompatibility> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}:white_check_mark: - There are no reported addon version compatibility issues."
      ));
    }

    let mut table = String::new();
    table.push_str(&format!(
      "{leading_whitespace}|   -   | Name  | Version | Default | Latest | Next Default | Next Latest |\n"
    ));
    table.push_str(&format!(
      "{leading_whitespace}| :---: | :---- | :-----: | :-----: | :----: | :----------: | :---------: |\n"
    ));

    for finding in self {
      table.push_str(&format!(
        "{}| {} | {} | {} | {} | {} | {} | {} |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.name,
        finding.version,
        finding.current_kubernetes_version.default,
        finding.current_kubernetes_version.latest,
        finding.target_kubernetes_version.default,
        finding.target_kubernetes_version.latest,
      ))
    }

    Some(table)
  }
}

/// Check for any version compatibility issues for the EKS addons enabled
pub(crate) async fn addon_version_compatibility(
  client: &EksClient,
  cluster_version: &str,
  addons: &[Addon],
) -> Result<Vec<AddonVersionCompatibility>, anyhow::Error> {
  let mut addon_versions = Vec::new();
  let target_k8s_version = format!("1.{}", version::parse_minor(cluster_version)? + 1);

  for addon in addons {
    let name = addon.addon_name().unwrap().to_owned();
    let version = addon.addon_version().unwrap().to_owned();

    let current_kubernetes_version = get_addon_versions(client, &name, cluster_version).await?;
    let target_kubernetes_version = get_addon_versions(client, &name, &target_k8s_version).await?;

    // TODO - why is this saying the if/else is the same?
    #[allow(clippy::if_same_then_else)]
    let remediation = if !target_kubernetes_version.supported_versions.contains(&version) {
      // The target Kubernetes version of addons does not support the current addon version, must update
      Some(finding::Remediation::Required)
    } else if !current_kubernetes_version.supported_versions.contains(&version) {
      // The current Kubernetes version of addons does not support the current addon version, must update
      Some(finding::Remediation::Required)
    } else if current_kubernetes_version.latest != version {
      // The current Kubernetes version of addons supports the current addon version, but it is not the latest
      Some(finding::Remediation::Recommended)
    } else {
      None
    };

    if let Some(remediation) = remediation {
      addon_versions.push(AddonVersionCompatibility {
        name,
        version,
        current_kubernetes_version,
        target_kubernetes_version,
        remediation,
        code: finding::Code::EKS005,
      })
    }
  }

  Ok(addon_versions)
}

/// Addon health issue data
///
/// Nearly identical to the SDK's `AddonIssue` but allows us to serialize/deserialize
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AddonHealthIssue {
  pub(crate) name: String,
  pub(crate) code: String,
  pub(crate) message: String,
  pub(crate) resource_ids: Vec<String>,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

impl Findings for Vec<AddonHealthIssue> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}:white_check_mark: - There are no reported addon health issues."
      ));
    }

    let mut table = String::new();
    table.push_str(&format!(
      "{leading_whitespace}|   -   | Name  | Code  | Message | Resource IDs |\n"
    ));
    table.push_str(&format!(
      "{leading_whitespace}| :---: | :---- | :---: | :------ | :----------- |\n"
    ));

    for finding in self {
      table.push_str(&format!(
        "{}| {} | {} | {} | {} | {} |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.name,
        finding.code,
        finding.message,
        finding.resource_ids.join(", ")
      ))
    }

    Some(table)
  }
}

pub(crate) async fn addon_health(addons: &[Addon]) -> Result<Vec<AddonHealthIssue>, anyhow::Error> {
  let health_issues = addons
    .iter()
    .flat_map(|addon| {
      let name = addon.addon_name().unwrap();
      let health = addon.health().unwrap();

      health
        .issues()
        .unwrap()
        .iter()
        .map(|issue| {
          let code = issue.code().unwrap();

          AddonHealthIssue {
            name: name.to_owned(),
            code: code.as_str().to_string(),
            message: issue.message().unwrap().to_owned(),
            resource_ids: issue.resource_ids().unwrap().to_owned(),
            remediation: finding::Remediation::Required,
            fcode: finding::Code::EKS004,
          }
        })
        .collect::<Vec<AddonHealthIssue>>()
    })
    .collect();

  Ok(health_issues)
}

pub(crate) async fn get_eks_managed_nodegroups(
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

/// Nodegroup health issue data
///
/// Nearly similar to the SDK's `NodegroupHealth` but flattened
/// and without `Option()`s to make it a bit more ergonomic here
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct NodegroupHealthIssue {
  pub(crate) name: String,
  pub(crate) code: String,
  pub(crate) message: String,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

impl Findings for Vec<NodegroupHealthIssue> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}:white_check_mark: - There are no reported nodegroup health issues."
      ));
    }

    let mut table = String::new();
    table.push_str(&format!("{leading_whitespace}|   -   | Name  | Code  | Message |\n"));
    table.push_str(&format!("{leading_whitespace}| :---: | :---- | :---: | :------ |\n"));

    for finding in self {
      table.push_str(&format!(
        "{}| {} | {} | {} | {} |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.name,
        finding.code,
        finding.message,
      ))
    }

    Some(table)
  }
}

/// Check for any reported health issues on EKS managed node groups
pub(crate) async fn eks_managed_nodegroup_health(
  nodegroups: &[Nodegroup],
) -> Result<Vec<NodegroupHealthIssue>, anyhow::Error> {
  let health_issues = nodegroups
    .iter()
    .flat_map(|nodegroup| {
      let name = nodegroup.nodegroup_name().unwrap();
      let health = nodegroup.health().unwrap();
      let issues = health.issues().unwrap();

      issues.iter().map(|issue| {
        let code = issue.code().unwrap();
        let message = issue.message().unwrap();

        NodegroupHealthIssue {
          name: name.to_owned(),
          code: code.as_str().to_owned(),
          message: message.to_owned(),
          remediation: finding::Remediation::Required,
          fcode: finding::Code::EKS003,
        }
      })
    })
    .collect();

  Ok(health_issues)
}

pub(crate) async fn get_self_managed_nodegroups(
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

pub(crate) async fn _get_fargate_profiles(
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

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LaunchTemplate {
  /// Name of the launch template
  pub(crate) name: String,
  /// The ID of the launch template
  pub(crate) id: String,
  /// The version of the launch template currently used/specified in the autoscaling group
  pub(crate) current_version: String,
  /// The latest version of the launch template
  pub(crate) latest_version: String,
}

async fn get_launch_template(client: &Ec2Client, id: &str) -> Result<LaunchTemplate, anyhow::Error> {
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

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ManagedNodeGroupUpdate {
  /// EKS managed node group name
  pub(crate) name: String,
  /// Name of the autoscaling group associated to the EKS managed node group
  pub(crate) autoscaling_group_name: String,
  /// Launch template controlled by users that influences the autoscaling group
  ///
  /// This distinction is important because we only consider the launch templates
  /// provided by users and not provided by EKS managed node group(s)
  pub(crate) launch_template: LaunchTemplate,
  // We do not consider launch configurations because you cannot determine if any
  // updates are pending like with launch templates and because they are being deprecated
  // https://docs.aws.amazon.com/autoscaling/ec2/userguide/launch-configurations.html
  // launch_configuration_name: Option<String>,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

impl Findings for Vec<ManagedNodeGroupUpdate> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}:white_check_mark: - There are no pending updates for the EKS managed nodegroup(s)"
      ));
    }

    let mut table = String::new();
    table.push_str(&format!(
      "{leading_whitespace}|       | Name  | Launch Template ID | Current Ver. | Latest Ver. |\n"
    ));
    table.push_str(&format!(
      "{leading_whitespace}| :---: | :---- | :----------------- | :----------- | :---------- |\n"
    ));

    for finding in self {
      table.push_str(&format!(
        "{}| {} | {} | {} | {} | {} |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.name,
        finding.launch_template.id,
        finding.launch_template.current_version,
        finding.launch_template.latest_version,
      ))
    }

    Some(table)
  }
}

pub(crate) async fn eks_managed_nodegroup_update(
  client: &Ec2Client,
  nodegroup: &Nodegroup,
) -> Result<Vec<ManagedNodeGroupUpdate>, anyhow::Error> {
  let launch_template_spec = nodegroup.launch_template();

  // On EKS managed node groups, there are between 1 and 2 launch templates that influence the node group.
  // If the user does not specify a launch template, EKS will provide its own template.
  // If the user does specify a launch template, EKS will merge the values from that template with its own template.
  // Therefore, the launch template shown on the autoscaling group is managed by EKS and reflective of showing
  // whether there are pending changes or not (pending changes due to launch template changes). Instead, we will only
  // check the launch template field of the EKS managed node group which is the user provided template, if there is one.
  match launch_template_spec {
    Some(launch_template_spec) => {
      let launch_template_id = launch_template_spec.id().unwrap().to_owned();
      let launch_template = get_launch_template(client, &launch_template_id).await?;

      let updates = nodegroup
        .resources()
        .unwrap()
        .auto_scaling_groups()
        .unwrap()
        .iter()
        .map(|asg| ManagedNodeGroupUpdate {
          name: nodegroup.nodegroup_name().unwrap().to_owned(),
          autoscaling_group_name: asg.name().unwrap().to_owned(),
          launch_template: launch_template.to_owned(),
          remediation: finding::Remediation::Recommended,
          fcode: finding::Code::EKS006,
        })
        // Only interested in those that are not using the latest version
        .filter(|asg| asg.launch_template.current_version != asg.launch_template.latest_version)
        .collect();
      Ok(updates)
    }
    None => Ok(vec![]),
  }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AutoscalingGroupUpdate {
  /// Autoscaling group name
  pub(crate) name: String,
  /// Launch template used by the autoscaling group
  pub(crate) launch_template: LaunchTemplate,
  // We do not consider launch configurations because you cannot determine if any
  // updates are pending like with launch templates and because they are being deprecated
  // https://docs.aws.amazon.com/autoscaling/ec2/userguide/launch-configurations.html
  // launch_configuration_name: Option<String>,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

impl Findings for Vec<AutoscalingGroupUpdate> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}:white_check_mark: - There are no pending updates for the self-managed nodegroup(s)"
      ));
    }

    let mut table = String::new();
    table.push_str(&format!(
      "{leading_whitespace}|       | Name  | Launch Template ID | Current Ver. | Latest Ver. |\n"
    ));
    table.push_str(&format!(
      "{leading_whitespace}| :---: | :---- | :----------------- | :----------- | :---------- |\n"
    ));

    for finding in self {
      table.push_str(&format!(
        "{}| {} | {} | {} | {} | {} |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.name,
        finding.launch_template.id,
        finding.launch_template.current_version,
        finding.launch_template.latest_version,
      ))
    }

    Some(table)
  }
}

/// Returns the autoscaling groups that are not using the latest launch template version
///
/// If there are pending changes, users do not necessarily need to make any changes prior to upgrading.
/// They should, however, be aware of the version currently in use and any changes that may be
/// deployed when updating the launch template for the new Kubernetes version. For example, if the
/// current launch template version is 3 and the latest version is 5, the user should be aware that
/// there may, or may not, be additional changes that were introduced in version 4 and 5 that might be
/// deployed when the launch template is updated to version 6 for the Kubernetes version upgrade. Ideally,
/// users should be on the latest version of the launch template prior to upgrading to avoid any surprises
/// or unexpected changes.
pub(crate) async fn self_managed_nodegroup_update(
  client: &Ec2Client,
  asg: &AutoScalingGroup,
) -> Result<Option<AutoscalingGroupUpdate>, anyhow::Error> {
  let name = asg.auto_scaling_group_name().unwrap().to_owned();
  let launch_template_id = asg.launch_template().unwrap().launch_template_id().unwrap().to_owned();
  let launch_template = get_launch_template(client, &launch_template_id).await?;

  // Only interested in those that are not using the latest version
  if launch_template.current_version != launch_template.latest_version {
    let update = AutoscalingGroupUpdate {
      name,
      launch_template,
      remediation: finding::Remediation::Recommended,
      fcode: finding::Code::EKS007,
    };
    Ok(Some(update))
  } else {
    Ok(None)
  }
}
