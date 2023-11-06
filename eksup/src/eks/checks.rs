use anyhow::{Context, Result};
use aws_sdk_autoscaling::types::AutoScalingGroup;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_eks::{
  types::{Addon, Cluster, Nodegroup},
  Client as EksClient,
};
use itertools::Itertools;
use kube::Client as K8sClient;
use serde::{Deserialize, Serialize};
use tabled::{
  settings::{locator::ByColumnName, Disable, Margin, Style},
  Table, Tabled,
};

use crate::{
  eks::resources,
  finding::{self, Findings},
  k8s,
  output::tabled_vec_to_string,
  version,
};

/// Cluster health issue data
///
/// Nearly identical to the SDK's `ClusterIssue` but allows us to serialize/deserialize
#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct ClusterHealthIssue {
  #[tabled(inline)]
  pub finding: finding::Finding,
  pub code: String,
  pub message: String,
  #[tabled(display_with = "tabled_vec_to_string")]
  pub resource_ids: Vec<String>,
}

impl Findings for Vec<ClusterHealthIssue> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - There are no reported health issues on the cluster control plane"
      ));
    }

    let mut table = Table::new(self);
    table
      .with(Margin::new(1, 0, 0, 0).fill('\t', 'x', 'x', 'x'))
      .with(Style::markdown());

    Ok(format!("{table}\n"))
  }

  fn to_stdout_table(&self) -> Result<String> {
    if self.is_empty() {
      return Ok("".to_owned());
    }

    let mut table = Table::new(self);
    table.with(Style::sharp());

    Ok(format!("{table}\n"))
  }
}

/// Check for any reported health issues on the cluster control plane
pub(crate) async fn cluster_health(cluster: &Cluster) -> Result<Vec<ClusterHealthIssue>> {
  let health = cluster.health();

  match health {
    Some(health) => Ok(
      health
        .issues()
        .iter()
        .filter_map(|issue| {
          issue.code.as_ref().map(|_| {
            let code = &issue.code().unwrap().to_owned();

            let remediation = finding::Remediation::Required;
            let finding = finding::Finding {
              code: finding::Code::EKS002,
              symbol: remediation.symbol(),
              remediation,
            };

            ClusterHealthIssue {
              finding,
              code: code.as_str().to_string(),
              message: issue.message().unwrap_or_default().to_string(),
              resource_ids: issue.resource_ids().to_owned(),
            }
          })
        })
        .collect(),
    ),
    None => Ok(vec![]),
  }
}

/// Subnet details that can affect upgrade behavior
#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct InsufficientSubnetIps {
  #[tabled(inline)]
  pub finding: finding::Finding,
  pub id: String,
  pub available_ips: i32,
}

impl Findings for Vec<InsufficientSubnetIps> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - There is sufficient IP space in the subnets provided"
      ));
    }

    let mut table = Table::new(self);
    table
      .with(Disable::column(ByColumnName::new("CHECK")))
      .with(Margin::new(1, 0, 0, 0).fill('\t', 'x', 'x', 'x'))
      .with(Style::markdown());

    Ok(format!("{table}\n"))
  }

  fn to_stdout_table(&self) -> Result<String> {
    if self.is_empty() {
      return Ok("".to_owned());
    }

    let mut table = Table::new(self);
    table.with(Style::sharp());

    Ok(format!("{table}\n"))
  }
}

pub(crate) async fn control_plane_ips(ec2_client: &Ec2Client, cluster: &Cluster) -> Result<Vec<InsufficientSubnetIps>> {
  let subnet_ids = match cluster.resources_vpc_config() {
    Some(vpc_config) => vpc_config.subnet_ids().to_owned(),
    None => return Ok(vec![]),
  };

  let subnet_ips = resources::get_subnet_ips(ec2_client, subnet_ids).await?;

  let availability_zone_ips: Vec<(String, i32)> = subnet_ips
    .iter()
    .group_by(|subnet| subnet.availability_zone_id.clone())
    .into_iter()
    .map(|(az, subnets)| {
      let total_ips = subnets.map(|subnet| subnet.available_ips).sum();
      (az, total_ips)
    })
    .collect();

  // There are at least 2 different availability zones with 5 or more IPs; no finding
  if availability_zone_ips
    .iter()
    .filter(|(_az, ips)| ips >= &5)
    .collect::<Vec<_>>()
    .len()
    >= 2
  {
    return Ok(vec![]);
  }

  let remediation = finding::Remediation::Required;
  let finding = finding::Finding {
    code: finding::Code::EKS001,
    symbol: remediation.symbol(),
    remediation,
  };

  Ok(
    availability_zone_ips
      .iter()
      .map(|(az, ips)| InsufficientSubnetIps {
        finding: finding.clone(),
        id: az.clone(),
        available_ips: *ips,
      })
      .collect(),
  )
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
) -> Result<Vec<InsufficientSubnetIps>> {
  let eniconfigs = k8s::get_eniconfigs(k8s_client).await?;
  if eniconfigs.is_empty() {
    return Ok(vec![]);
  }

  let subnet_ids = eniconfigs
    .iter()
    .map(|eniconfig| eniconfig.spec.subnet.as_ref().unwrap().to_owned())
    .collect();

  let subnet_ips = resources::get_subnet_ips(ec2_client, subnet_ids).await?;
  let available_ips: i32 = subnet_ips.iter().map(|subnet| subnet.available_ips).sum();

  if available_ips >= recommended_ips {
    return Ok(vec![]);
  }

  let remediation = if available_ips >= required_ips {
    finding::Remediation::Required
  } else {
    finding::Remediation::Recommended
  };

  let finding = finding::Finding {
    code: finding::Code::AWS002,
    symbol: remediation.symbol(),
    remediation,
  };

  Ok(
    subnet_ips
      .iter()
      .group_by(|subnet| subnet.availability_zone_id.clone())
      .into_iter()
      .map(|(az, subnets)| InsufficientSubnetIps {
        finding: finding.clone(),
        id: az,
        available_ips: subnets.map(|subnet| subnet.available_ips).sum(),
      })
      .collect(),
  )
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
#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct AddonVersionCompatibility {
  #[tabled(inline)]
  pub finding: finding::Finding,
  pub name: String,
  /// The current version of the add-on
  #[tabled(rename = "CURRENT")]
  pub version: String,
  /// The default and latest add-on versions for the current Kubernetes version
  #[tabled(skip)]
  pub current_kubernetes_version: resources::AddonVersion,
  /// The default and latest add-on versions for the target Kubernetes version
  #[tabled(inline)]
  pub target_kubernetes_version: resources::AddonVersion,
}

impl Findings for Vec<AddonVersionCompatibility> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - There are no reported addon version compatibility issues."
      ));
    }

    let mut table = Table::new(self);
    table
      .with(Disable::column(ByColumnName::new("CHECK")))
      .with(Margin::new(1, 0, 0, 0).fill('\t', 'x', 'x', 'x'))
      .with(Style::markdown());

    Ok(format!("{table}\n"))
  }

  fn to_stdout_table(&self) -> Result<String> {
    if self.is_empty() {
      return Ok("".to_owned());
    }

    let mut table = Table::new(self);
    table.with(Style::sharp());

    Ok(format!("{table}\n"))
  }
}

/// Check for any version compatibility issues for the EKS addons enabled
pub(crate) async fn addon_version_compatibility(
  client: &EksClient,
  cluster_version: &str,
  addons: &[Addon],
) -> Result<Vec<AddonVersionCompatibility>> {
  let mut addon_versions = Vec::new();
  let target_k8s_version = format!("1.{}", version::parse_minor(cluster_version)? + 1);

  for addon in addons {
    let name = addon.addon_name().unwrap_or_default().to_owned();
    let version = addon.addon_version().unwrap_or_default().to_owned();

    let current_kubernetes_version = resources::get_addon_versions(client, &name, cluster_version).await?;
    let target_kubernetes_version = resources::get_addon_versions(client, &name, &target_k8s_version).await?;

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
      let finding = finding::Finding {
        code: finding::Code::EKS005,
        symbol: remediation.symbol(),
        remediation,
      };

      addon_versions.push(AddonVersionCompatibility {
        finding,
        name,
        version,
        current_kubernetes_version,
        target_kubernetes_version,
      })
    }
  }

  Ok(addon_versions)
}

/// Addon health issue data
///
/// Nearly identical to the SDK's `AddonIssue` but allows us to serialize/deserialize
#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct AddonHealthIssue {
  #[tabled(inline)]
  pub finding: finding::Finding,
  pub name: String,
  pub code: String,
  pub message: String,
  #[tabled(display_with = "tabled_vec_to_string")]
  pub resource_ids: Vec<String>,
}

impl Findings for Vec<AddonHealthIssue> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - There are no reported addon health issues."
      ));
    }

    let mut table = Table::new(self);
    table
      .with(Disable::column(ByColumnName::new("CHECK")))
      .with(Margin::new(1, 0, 0, 0).fill('\t', 'x', 'x', 'x'))
      .with(Style::markdown());

    Ok(format!("{table}\n"))
  }

  fn to_stdout_table(&self) -> Result<String> {
    if self.is_empty() {
      return Ok("".to_owned());
    }

    let mut table = Table::new(self);
    table.with(Style::sharp());

    Ok(format!("{table}\n"))
  }
}

pub(crate) async fn addon_health(addons: &[Addon]) -> Result<Vec<AddonHealthIssue>> {
  let health_issues = addons
    .iter()
    .flat_map(|addon| {
      let name = addon.addon_name().unwrap_or_default();

      match addon.health() {
        Some(health) => health
          .issues()
          .iter()
          .filter_map(|issue| {
            issue.code.as_ref().map(|_| {
              let code = issue.code().unwrap();

              let remediation = finding::Remediation::Required;
              let finding = finding::Finding {
                code: finding::Code::EKS004,
                symbol: remediation.symbol(),
                remediation,
              };

              AddonHealthIssue {
                finding,
                name: name.to_owned(),
                code: code.as_str().to_string(),
                message: issue.message().unwrap_or_default().to_owned(),
                resource_ids: issue.resource_ids().to_owned(),
              }
            })
          })
          .collect::<Vec<AddonHealthIssue>>(),
        None => vec![],
      }
    })
    .collect();

  Ok(health_issues)
}

/// Nodegroup health issue data
///
/// Nearly similar to the SDK's `NodegroupHealth` but flattened
/// and without `Option()`s to make it a bit more ergonomic here
#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct NodegroupHealthIssue {
  #[tabled(inline)]
  pub finding: finding::Finding,
  pub name: String,
  pub code: String,
  pub message: String,
}

impl Findings for Vec<NodegroupHealthIssue> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - There are no reported nodegroup health issues."
      ));
    }

    let mut table = Table::new(self);
    table
      .with(Disable::column(ByColumnName::new("CHECK")))
      .with(Margin::new(1, 0, 0, 0).fill('\t', 'x', 'x', 'x'))
      .with(Style::markdown());

    Ok(format!("{table}\n"))
  }

  fn to_stdout_table(&self) -> Result<String> {
    if self.is_empty() {
      return Ok("".to_owned());
    }

    let mut table = Table::new(self);
    table.with(Style::sharp());

    Ok(format!("{table}\n"))
  }
}

/// Check for any reported health issues on EKS managed node groups
pub(crate) async fn eks_managed_nodegroup_health(nodegroups: &[Nodegroup]) -> Result<Vec<NodegroupHealthIssue>> {
  let health_issues = nodegroups
    .iter()
    .flat_map(|nodegroup| {
      let name = nodegroup.nodegroup_name().unwrap_or_default();

      match nodegroup.health() {
        Some(health) => health
          .issues()
          .iter()
          .filter_map(|issue| {
            issue.code.as_ref().map(|_| {
              let code = &issue.code().unwrap().to_owned();

              let remediation = finding::Remediation::Required;
              let finding = finding::Finding {
                code: finding::Code::EKS003,
                symbol: remediation.symbol(),
                remediation,
              };

              NodegroupHealthIssue {
                finding,
                name: name.to_owned(),
                code: code.as_str().to_string(),
                message: issue.message().unwrap_or_default().to_owned(),
              }
            })
          })
          .collect::<Vec<NodegroupHealthIssue>>(),
        None => vec![],
      }
    })
    .collect();

  Ok(health_issues)
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct ManagedNodeGroupUpdate {
  #[tabled(inline)]
  pub finding: finding::Finding,
  /// EKS managed node group name
  #[tabled(rename = "MANAGED NODEGROUP")]
  pub name: String,
  /// Name of the autoscaling group associated to the EKS managed node group
  #[tabled(skip)]
  pub autoscaling_group_name: String,
  /// Launch template controlled by users that influences the autoscaling group
  ///
  /// This distinction is important because we only consider the launch templates
  /// provided by users and not provided by EKS managed node group(s)
  #[tabled(inline)]
  pub launch_template: resources::LaunchTemplate,
  // We do not consider launch configurations because you cannot determine if any
  // updates are pending like with launch templates and because they are being deprecated
  // https://docs.aws.amazon.com/autoscaling/ec2/userguide/launch-configurations.html
  // launch_configuration_name: Option<String>,
}

impl Findings for Vec<ManagedNodeGroupUpdate> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - There are no pending updates for the EKS managed nodegroup(s)"
      ));
    }

    let mut table = Table::new(self);
    table
      .with(Disable::column(ByColumnName::new("CHECK")))
      .with(Margin::new(1, 0, 0, 0).fill('\t', 'x', 'x', 'x'))
      .with(Style::markdown());

    Ok(format!("{table}\n"))
  }

  fn to_stdout_table(&self) -> Result<String> {
    if self.is_empty() {
      return Ok("".to_owned());
    }

    let mut table = Table::new(self);
    table.with(Style::sharp());

    Ok(format!("{table}\n"))
  }
}

pub(crate) async fn eks_managed_nodegroup_update(
  client: &Ec2Client,
  nodegroup: &Nodegroup,
) -> Result<Vec<ManagedNodeGroupUpdate>> {
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
      let launch_template = resources::get_launch_template(client, &launch_template_id).await?;

      match nodegroup.resources() {
        Some(resources) => {
          let updates = resources
            .auto_scaling_groups()
            .iter()
            .map(|asg| {
              let remediation = finding::Remediation::Recommended;
              let finding = finding::Finding {
                code: finding::Code::EKS006,
                symbol: remediation.symbol(),
                remediation,
              };

              ManagedNodeGroupUpdate {
                finding,
                name: nodegroup.nodegroup_name().unwrap_or_default().to_owned(),
                autoscaling_group_name: asg.name().unwrap_or_default().to_owned(),
                launch_template: launch_template.to_owned(),
              }
            })
            // Only interested in those that are not using the latest version
            .filter(|asg| asg.launch_template.current_version != asg.launch_template.latest_version)
            .collect();

          Ok(updates)
        }
        None => Ok(vec![]),
      }
    }
    None => Ok(vec![]),
  }
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct AutoscalingGroupUpdate {
  #[tabled(inline)]
  pub finding: finding::Finding,
  /// Autoscaling group name
  #[tabled(rename = "AUTOSCALING GROUP")]
  pub name: String,
  /// Launch template used by the autoscaling group
  #[tabled(inline)]
  pub launch_template: resources::LaunchTemplate,
  // We do not consider launch configurations because you cannot determine if any
  // updates are pending like with launch templates and because they are being deprecated
  // https://docs.aws.amazon.com/autoscaling/ec2/userguide/launch-configurations.html
  // launch_configuration_name: Option<String>,
}

impl Findings for Vec<AutoscalingGroupUpdate> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - There are no pending updates for the self-managed nodegroup(s)"
      ));
    }

    let mut table = Table::new(self);
    table
      .with(Disable::column(ByColumnName::new("CHECK")))
      .with(Margin::new(1, 0, 0, 0).fill('\t', 'x', 'x', 'x'))
      .with(Style::markdown());

    Ok(format!("{table}\n"))
  }

  fn to_stdout_table(&self) -> Result<String> {
    if self.is_empty() {
      return Ok("".to_owned());
    }

    let mut table = Table::new(self);
    table.with(Style::sharp());

    Ok(format!("{table}\n"))
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
) -> Result<Option<AutoscalingGroupUpdate>> {
  let name = asg.auto_scaling_group_name().unwrap_or_default().to_owned();
  let lt_spec = asg
    .launch_template()
    .context("Launch template not found, launch configuration is not supported")?;
  let launch_template =
    resources::get_launch_template(client, lt_spec.launch_template_id().unwrap_or_default()).await?;

  // Only interested in those that are not using the latest version
  if launch_template.current_version != launch_template.latest_version {
    let remediation = finding::Remediation::Recommended;
    let finding = finding::Finding {
      code: finding::Code::EKS007,
      symbol: remediation.symbol(),
      remediation,
    };

    let update = AutoscalingGroupUpdate {
      finding,
      name,
      launch_template,
    };
    Ok(Some(update))
  } else {
    Ok(None)
  }
}
