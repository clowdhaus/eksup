use std::collections::HashSet;

use anyhow::Result;
use aws_sdk_autoscaling::model::AutoScalingGroup;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_eks::{
  model::{Addon, Cluster, Nodegroup},
  Client as EksClient,
};
use kube::Client as K8sClient;
use serde::{Deserialize, Serialize};
use tabled::{format::Format, object::Rows, Modify, Style, Table, Tabled};

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
  #[tabled(rename = "CHECK")]
  pub fcode: finding::Code,
  pub remediation: finding::Remediation,
  pub code: String,
  pub message: String,
  #[tabled(display_with = "tabled_vec_to_string")]
  pub resource_ids: Vec<String>,
}

impl Findings for Vec<ClusterHealthIssue> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}✅ - There are no reported health issues on the cluster control plane"
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
        "{}| {} | `{}` | `{}` | {} |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.code,
        finding.message,
        finding
          .resource_ids
          .iter()
          .map(|f| format!("`{f}`"))
          .collect::<Vec<String>>()
          .join(", "),
      ))
    }

    Some(table)
  }

  fn to_stdout_table(&self) -> Result<String> {
    if self.is_empty() {
      return Ok("".to_owned());
    }

    let mut table = Table::new(self);
    let style = Style::blank();
    table
      .with(style)
      .with(Modify::new(Rows::first()).with(Format::new(|s| s.to_uppercase())));

    Ok(table.to_string())
  }
}

/// Check for any reported health issues on the cluster control plane
pub(crate) async fn cluster_health(cluster: &Cluster) -> Result<Vec<ClusterHealthIssue>> {
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

/// Subnet details that can affect upgrade behavior
#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct InsufficientSubnetIps {
  #[tabled(rename = "CHECK")]
  pub fcode: finding::Code,
  pub remediation: finding::Remediation,
  #[tabled(display_with = "tabled_vec_to_string")]
  pub ids: Vec<String>,
  pub available_ips: i32,
}

impl Findings for Option<InsufficientSubnetIps> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    match self {
      Some(finding) => {
        let mut table = String::new();
        table.push_str(&format!(
          "{leading_whitespace}|   -   | Subnet IDs  | Available IPs |\n"
        ));
        table.push_str(&format!(
          "{leading_whitespace}| :---: | :---------- | :-----------: |\n"
        ));
        table.push_str(&format!(
          "{}| {} | {} | `{}` |\n",
          leading_whitespace,
          finding.remediation.symbol(),
          finding
            .ids
            .iter()
            .map(|f| format!("`{f}`"))
            .collect::<Vec<String>>()
            .join(", "),
          finding.available_ips,
        ));

        Some(table)
      }
      None => Some(format!(
        "{leading_whitespace}✅ - There is sufficient IP space in the subnets provided"
      )),
    }
  }

  fn to_stdout_table(&self) -> Result<String> {
    match self {
      None => Ok("".to_owned()),
      Some(finding) => {
        let mut table = Table::new(vec![finding]);
        table.with(Style::blank());

        Ok(table.to_string())
      }
    }
  }
}

pub(crate) async fn control_plane_ips(
  ec2_client: &Ec2Client,
  cluster: &Cluster,
) -> Result<Option<InsufficientSubnetIps>> {
  let subnet_ids = cluster.resources_vpc_config().unwrap().subnet_ids().unwrap().to_owned();

  let subnet_ips = resources::get_subnet_ips(ec2_client, subnet_ids).await?;
  if subnet_ips.available_ips >= 5 {
    return Ok(None);
  }

  let finding = InsufficientSubnetIps {
    ids: subnet_ips.ids,
    available_ips: subnet_ips.available_ips,
    remediation: finding::Remediation::Required,
    fcode: finding::Code::EKS001,
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
) -> Result<Option<InsufficientSubnetIps>> {
  let eniconfigs = k8s::get_eniconfigs(k8s_client).await?;
  if eniconfigs.is_empty() {
    return Ok(None);
  }

  let subnet_ids = eniconfigs
    .iter()
    .map(|eniconfig| eniconfig.spec.subnet.as_ref().unwrap().to_owned())
    .collect();

  let subnet_ips = resources::get_subnet_ips(ec2_client, subnet_ids).await?;

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
    fcode: finding::Code::AWS002,
  };

  Ok(Some(finding))
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
async fn get_addon_versions(client: &EksClient, name: &str, kubernetes_version: &str) -> Result<AddonVersion> {
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
  #[tabled(rename = "CHECK")]
  pub fcode: finding::Code,
  pub remediation: finding::Remediation,
  pub name: String,
  /// The current version of the add-on
  pub version: String,
  /// The default and latest add-on versions for the current Kubernetes version
  #[tabled(inline)]
  pub current_kubernetes_version: AddonVersion,
  /// The default and latest add-on versions for the target Kubernetes version
  #[tabled(inline)]
  pub target_kubernetes_version: AddonVersion,
}

impl Findings for Vec<AddonVersionCompatibility> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}✅ - There are no reported addon version compatibility issues."
      ));
    }

    let mut table = String::new();
    table.push_str(&format!(
      "{leading_whitespace}|   -   | Name  | Version | Next Default | Next Latest |\n"
    ));
    table.push_str(&format!(
      "{leading_whitespace}| :---: | :---- | :-----: | :----------: | :---------: |\n"
    ));

    for finding in self {
      table.push_str(&format!(
        "{}| {} | `{}` | `{}` | `{}` | `{}` |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.name,
        finding.version,
        finding.target_kubernetes_version.default,
        finding.target_kubernetes_version.latest,
      ))
    }

    Some(table)
  }

  fn to_stdout_table(&self) -> Result<String> {
    if self.is_empty() {
      return Ok("".to_owned());
    }

    let mut table = Table::new(self);
    table.with(Style::sharp());

    Ok(format!("{}\n", table.to_string()))
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
        fcode: finding::Code::EKS005,
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
  #[tabled(rename = "CHECK")]
  pub fcode: finding::Code,
  pub remediation: finding::Remediation,
  pub name: String,
  pub code: String,
  pub message: String,
  #[tabled(display_with = "tabled_vec_to_string")]
  pub resource_ids: Vec<String>,
}

impl Findings for Vec<AddonHealthIssue> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}✅ - There are no reported addon health issues."
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
        "{}| {} | `{}` | `{}` | `{}` | {} |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.name,
        finding.code,
        finding.message,
        finding
          .resource_ids
          .iter()
          .map(|f| format!("`{f}`"))
          .collect::<Vec<String>>()
          .join(", "),
      ))
    }

    Some(table)
  }

  fn to_stdout_table(&self) -> Result<String> {
    if self.is_empty() {
      return Ok("".to_owned());
    }

    let mut table = Table::new(self);
    table.with(Style::blank());

    Ok(table.to_string())
  }
}

pub(crate) async fn addon_health(addons: &[Addon]) -> Result<Vec<AddonHealthIssue>> {
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

/// Nodegroup health issue data
///
/// Nearly similar to the SDK's `NodegroupHealth` but flattened
/// and without `Option()`s to make it a bit more ergonomic here
#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub(crate) struct NodegroupHealthIssue {
  #[tabled(rename = "CHECK")]
  pub(crate) fcode: finding::Code,
  pub(crate) remediation: finding::Remediation,
  pub(crate) name: String,
  pub(crate) code: String,
  pub(crate) message: String,
}

impl Findings for Vec<NodegroupHealthIssue> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}✅ - There are no reported nodegroup health issues."
      ));
    }

    let mut table = String::new();
    table.push_str(&format!("{leading_whitespace}|   -   | Name  | Code  | Message |\n"));
    table.push_str(&format!("{leading_whitespace}| :---: | :---- | :---: | :------ |\n"));

    for finding in self {
      table.push_str(&format!(
        "{}| {} | `{}` | `{}` | `{}` |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.name,
        finding.code,
        finding.message,
      ))
    }

    Some(table)
  }

  fn to_stdout_table(&self) -> Result<String> {
    if self.is_empty() {
      return Ok("".to_owned());
    }

    let mut table = Table::new(self);
    table.with(Style::blank());

    Ok(table.to_string())
  }
}

/// Check for any reported health issues on EKS managed node groups
pub(crate) async fn eks_managed_nodegroup_health(nodegroups: &[Nodegroup]) -> Result<Vec<NodegroupHealthIssue>> {
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

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub(crate) struct ManagedNodeGroupUpdate {
  #[tabled(rename = "CHECK")]
  pub(crate) fcode: finding::Code,
  pub(crate) remediation: finding::Remediation,
  /// EKS managed node group name
  pub(crate) name: String,
  /// Name of the autoscaling group associated to the EKS managed node group
  pub(crate) autoscaling_group_name: String,
  /// Launch template controlled by users that influences the autoscaling group
  ///
  /// This distinction is important because we only consider the launch templates
  /// provided by users and not provided by EKS managed node group(s)
  #[tabled(inline)]
  pub(crate) launch_template: resources::LaunchTemplate,
  // We do not consider launch configurations because you cannot determine if any
  // updates are pending like with launch templates and because they are being deprecated
  // https://docs.aws.amazon.com/autoscaling/ec2/userguide/launch-configurations.html
  // launch_configuration_name: Option<String>,
}

impl Findings for Vec<ManagedNodeGroupUpdate> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}✅ - There are no pending updates for the EKS managed nodegroup(s)"
      ));
    }

    let mut table = String::new();
    table.push_str(&format!(
      "{leading_whitespace}|   -   | MNG Name  | Launch Template ID | Current | Latest |\n"
    ));
    table.push_str(&format!(
      "{leading_whitespace}| :---: | :-------- | :----------------- | :-----: | :----: |\n"
    ));

    for finding in self {
      table.push_str(&format!(
        "{}| {} | `{}` | `{}` | `{}` | `{}` |\n",
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

  fn to_stdout_table(&self) -> Result<String> {
    if self.is_empty() {
      return Ok("".to_owned());
    }

    let mut table = Table::new(self);
    table.with(Style::blank());

    Ok(table.to_string())
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

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub(crate) struct AutoscalingGroupUpdate {
  #[tabled(rename = "CHECK")]
  pub(crate) fcode: finding::Code,
  pub(crate) remediation: finding::Remediation,
  /// Autoscaling group name
  pub(crate) name: String,
  /// Launch template used by the autoscaling group
  #[tabled(inline)]
  pub(crate) launch_template: resources::LaunchTemplate,
  // We do not consider launch configurations because you cannot determine if any
  // updates are pending like with launch templates and because they are being deprecated
  // https://docs.aws.amazon.com/autoscaling/ec2/userguide/launch-configurations.html
  // launch_configuration_name: Option<String>,
}

impl Findings for Vec<AutoscalingGroupUpdate> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}✅ - There are no pending updates for the self-managed nodegroup(s)"
      ));
    }

    let mut table = String::new();
    table.push_str(&format!(
      "{leading_whitespace}|   -   | ASG Name | Launch Template ID | Current | Latest |\n"
    ));
    table.push_str(&format!(
      "{leading_whitespace}| :---: | :------- | :----------------- | :-----: | :----: |\n"
    ));

    for finding in self {
      table.push_str(&format!(
        "{}| {} | `{}` | `{}` | `{}` | `{}` |\n",
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

  fn to_stdout_table(&self) -> Result<String> {
    if self.is_empty() {
      return Ok("".to_owned());
    }

    let mut table = Table::new(self);
    table.with(Style::blank());

    Ok(table.to_string())
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
  let name = asg.auto_scaling_group_name().unwrap().to_owned();
  let launch_template_id = asg.launch_template().unwrap().launch_template_id().unwrap().to_owned();
  let launch_template = resources::get_launch_template(client, &launch_template_id).await?;

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
