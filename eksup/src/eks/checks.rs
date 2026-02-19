use std::collections::HashMap;

use anyhow::Result;
use aws_sdk_autoscaling::types::AutoScalingGroup;
use aws_sdk_eks::types::{Addon, AmiTypes, Cluster, Nodegroup};
use serde::{Deserialize, Serialize};
use tabled::{
  Table, Tabled,
  settings::{Margin, Style},
};

use crate::{
  eks::resources,
  finding::{self, Code, Finding, Findings, Remediation},
  output::tabled_vec_to_string,
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
  #[tabled(display = "tabled_vec_to_string")]
  pub resource_ids: Vec<String>,
}

// Manual impl (not using impl_findings! macro) because the markdown table
// intentionally keeps the CHECK column visible, unlike all other finding types.
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
pub(crate) fn cluster_health(cluster: &Cluster) -> Result<Vec<ClusterHealthIssue>> {
  let health = cluster.health();

  match health {
    Some(health) => Ok(
      health
        .issues()
        .iter()
        .filter_map(|issue| {
          issue.code.as_ref().map(|code| ClusterHealthIssue {
            finding: Finding::new(Code::EKS002, Remediation::Required),
            code: code.as_str().to_string(),
            message: issue.message().unwrap_or_default().to_string(),
            resource_ids: issue.resource_ids().to_owned(),
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

finding::impl_findings!(InsufficientSubnetIps, "✅ - There is sufficient IP space in the subnets provided");

pub(crate) fn control_plane_ips(subnet_ips: &[resources::VpcSubnet]) -> Vec<InsufficientSubnetIps> {
  let mut az_ips: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
  for subnet in subnet_ips {
    *az_ips.entry(subnet.availability_zone_id.clone()).or_default() += subnet.available_ips;
  }
  let mut availability_zone_ips: Vec<(String, i32)> = az_ips.into_iter().collect();
  availability_zone_ips.sort_by(|a, b| a.0.cmp(&b.0));

  // There are at least 2 different availability zones with 5 or more IPs; no finding
  if availability_zone_ips
    .iter()
    .filter(|(_az, ips)| ips >= &5)
    .count()
    >= 2
  {
    return vec![];
  }

  let finding = Finding::new(Code::EKS001, Remediation::Required);

  availability_zone_ips
    .iter()
    .map(|(az, ips)| InsufficientSubnetIps {
      finding: finding.clone(),
      id: az.clone(),
      available_ips: *ips,
    })
    .collect()
}

/// Check if the subnets used by the pods will support an upgrade
///
/// This checks for the `ENIConfig` custom resource that is used to configure
/// the AWS VPC CNI for custom networking. The subnet listed for each ENIConfig
/// is queried for its relevant data used to report on the available IPs
pub(crate) fn pod_ips(
  subnet_ips: &[resources::VpcSubnet],
  required_ips: i32,
  recommended_ips: i32,
) -> Vec<InsufficientSubnetIps> {
  if subnet_ips.is_empty() {
    return vec![];
  }

  let available_ips: i32 = subnet_ips.iter().map(|subnet| subnet.available_ips).sum();

  if available_ips >= recommended_ips {
    return vec![];
  }

  let remediation = if available_ips < required_ips {
    Remediation::Required
  } else {
    Remediation::Recommended
  };

  let finding = Finding::new(Code::AWS002, remediation);

  let mut az_ips: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
  for subnet in subnet_ips {
    *az_ips.entry(subnet.availability_zone_id.clone()).or_default() += subnet.available_ips;
  }

  let mut sorted_ips: Vec<(String, i32)> = az_ips.into_iter().collect();
  sorted_ips.sort_by(|a, b| a.0.cmp(&b.0));

  sorted_ips
    .into_iter()
    .map(|(az, ips)| InsufficientSubnetIps {
      finding: finding.clone(),
      id: az,
      available_ips: ips,
    })
    .collect()
}

/// Check available IPs in data plane subnets (nodegroup or Fargate profile subnets)
///
/// During an upgrade, the rolling-update/surge process requires additional IPs.
/// If the subnets used by a nodegroup or Fargate profile are running low,
/// the upgrade may fail or be unable to launch replacement nodes/pods.
pub(crate) fn data_plane_ips(
  subnet_ips: &[resources::VpcSubnet],
  required_ips: i32,
  recommended_ips: i32,
) -> Vec<InsufficientSubnetIps> {
  if subnet_ips.is_empty() {
    return vec![];
  }

  let available_ips: i32 = subnet_ips.iter().map(|s| s.available_ips).sum();

  if available_ips >= recommended_ips {
    return vec![];
  }

  let remediation = if available_ips < required_ips {
    Remediation::Required
  } else {
    Remediation::Recommended
  };

  let finding = Finding::new(Code::AWS001, remediation);

  let mut az_ips: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
  for subnet in subnet_ips {
    *az_ips.entry(subnet.availability_zone_id.clone()).or_default() += subnet.available_ips;
  }

  let mut sorted_ips: Vec<(String, i32)> = az_ips.into_iter().collect();
  sorted_ips.sort_by(|a, b| a.0.cmp(&b.0));

  sorted_ips
    .into_iter()
    .map(|(az, ips)| InsufficientSubnetIps {
      finding: finding.clone(),
      id: az,
      available_ips: ips,
    })
    .collect()
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

finding::impl_findings!(AddonVersionCompatibility, "✅ - There are no reported addon version compatibility issues.");

/// Check for any version compatibility issues for the EKS addons enabled
pub(crate) fn addon_version_compatibility(
  addons: &[Addon],
  current_versions: &HashMap<String, resources::AddonVersion>,
  target_versions: &HashMap<String, resources::AddonVersion>,
) -> Vec<AddonVersionCompatibility> {
  let mut addon_findings = Vec::new();

  for addon in addons {
    let name = addon.addon_name().unwrap_or_default().to_owned();
    let version = addon.addon_version().unwrap_or_default().to_owned();

    let current_kubernetes_version = match current_versions.get(&name) {
      Some(v) => v.clone(),
      None => continue,
    };
    let target_kubernetes_version = match target_versions.get(&name) {
      Some(v) => v.clone(),
      None => continue,
    };

    let remediation = if !target_kubernetes_version.supported_versions.contains(&version)
      || !current_kubernetes_version.supported_versions.contains(&version)
    {
      Some(Remediation::Required)
    } else if current_kubernetes_version.latest != version {
      Some(Remediation::Recommended)
    } else {
      None
    };

    if let Some(remediation) = remediation {
      addon_findings.push(AddonVersionCompatibility {
        finding: Finding::new(Code::EKS005, remediation),
        name,
        version,
        current_kubernetes_version,
        target_kubernetes_version,
      })
    }
  }

  addon_findings
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
  #[tabled(display = "tabled_vec_to_string")]
  pub resource_ids: Vec<String>,
}

finding::impl_findings!(AddonHealthIssue, "✅ - There are no reported addon health issues.");

pub(crate) fn addon_health(addons: &[Addon]) -> Result<Vec<AddonHealthIssue>> {
  let health_issues = addons
    .iter()
    .flat_map(|addon| {
      let name = addon.addon_name().unwrap_or_default();

      match addon.health() {
        Some(health) => health
          .issues()
          .iter()
          .filter_map(|issue| {
            issue.code.as_ref().map(|code| AddonHealthIssue {
              finding: Finding::new(Code::EKS004, Remediation::Required),
              name: name.to_owned(),
              code: code.as_str().to_string(),
              message: issue.message().unwrap_or_default().to_owned(),
              resource_ids: issue.resource_ids().to_owned(),
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

finding::impl_findings!(NodegroupHealthIssue, "✅ - There are no reported nodegroup health issues.");

/// Check for any reported health issues on EKS managed node groups
pub(crate) fn eks_managed_nodegroup_health(nodegroups: &[Nodegroup]) -> Result<Vec<NodegroupHealthIssue>> {
  let health_issues = nodegroups
    .iter()
    .flat_map(|nodegroup| {
      let name = nodegroup.nodegroup_name().unwrap_or_default();

      match nodegroup.health() {
        Some(health) => health
          .issues()
          .iter()
          .filter_map(|issue| {
            issue.code.as_ref().map(|code| NodegroupHealthIssue {
              finding: Finding::new(Code::EKS003, Remediation::Required),
              name: name.to_owned(),
              code: code.as_str().to_string(),
              message: issue.message().unwrap_or_default().to_owned(),
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
}

finding::impl_findings!(ManagedNodeGroupUpdate, "✅ - There are no pending updates for the EKS managed nodegroup(s)");

pub(crate) fn eks_managed_nodegroup_update(
  nodegroup: &Nodegroup,
  launch_template: Option<&resources::LaunchTemplate>,
) -> Vec<ManagedNodeGroupUpdate> {
  let launch_template = match launch_template {
    Some(lt) => lt,
    None => return vec![],
  };

  match nodegroup.resources() {
    Some(resources) => {
      resources
        .auto_scaling_groups()
        .iter()
        .map(|asg| {
          ManagedNodeGroupUpdate {
            finding: Finding::new(Code::EKS006, Remediation::Recommended),
            name: nodegroup.nodegroup_name().unwrap_or_default().to_owned(),
            autoscaling_group_name: asg.name().unwrap_or_default().to_owned(),
            launch_template: launch_template.to_owned(),
          }
        })
        .filter(|asg| asg.launch_template.current_version != asg.launch_template.latest_version)
        .collect()
    }
    None => vec![],
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
}

finding::impl_findings!(AutoscalingGroupUpdate, "✅ - There are no pending updates for the self-managed nodegroup(s)");

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
pub(crate) fn self_managed_nodegroup_update(
  asg: &AutoScalingGroup,
  launch_template: &resources::LaunchTemplate,
) -> Option<AutoscalingGroupUpdate> {
  let name = asg.auto_scaling_group_name().unwrap_or_default().to_owned();

  if launch_template.current_version != launch_template.latest_version {
    Some(AutoscalingGroupUpdate {
      finding: Finding::new(Code::EKS007, Remediation::Recommended),
      name,
      launch_template: launch_template.to_owned(),
    })
  } else {
    None
  }
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct Al2AmiDeprecation {
  #[tabled(inline)]
  pub finding: finding::Finding,
  pub name: String,
  #[tabled(rename = "AMI TYPE")]
  pub ami_type: String,
}

finding::impl_findings!(Al2AmiDeprecation, "✅ - No EKS managed nodegroups are using deprecated AL2 AMI types");

/// Check for EKS managed nodegroups using AL2 AMI types which are deprecated in 1.32 and
/// no longer supported starting in 1.33
pub(crate) fn al2_ami_deprecation(nodegroups: &[Nodegroup], target_minor: i32) -> Result<Vec<Al2AmiDeprecation>> {
  if target_minor < 32 {
    return Ok(vec![]);
  }

  let remediation = if target_minor >= 33 {
    Remediation::Required
  } else {
    Remediation::Recommended
  };

  let mut findings = Vec::new();
  for nodegroup in nodegroups {
    let ami_type = match nodegroup.ami_type() {
      Some(ami) => ami,
      None => continue,
    };

    let is_al2 = matches!(ami_type, AmiTypes::Al2X8664 | AmiTypes::Al2Arm64 | AmiTypes::Al2X8664Gpu);
    if is_al2 {
      findings.push(Al2AmiDeprecation {
        finding: Finding::new(Code::EKS008, remediation.clone()),
        name: nodegroup.nodegroup_name().unwrap_or_default().to_owned(),
        ami_type: ami_type.as_str().to_string(),
      });
    }
  }

  Ok(findings)
}

#[cfg(test)]
mod tests {
  use super::*;
  use aws_sdk_eks::types::{
    Addon, AddonHealth, AddonIssue, AddonIssueCode, AmiTypes, Cluster, ClusterHealth, ClusterIssue,
    ClusterIssueCode, Issue, Nodegroup, NodegroupHealth, NodegroupIssueCode,
  };

  // ---------- cluster_health ----------

  #[test]
  fn cluster_health_no_issues() {
    let cluster = Cluster::builder()
      .health(ClusterHealth::builder().build())
      .build();

    let result = cluster_health(&cluster).unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn cluster_health_none_health() {
    // Cluster with no health field set at all
    let cluster = Cluster::builder().build();

    let result = cluster_health(&cluster).unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn cluster_health_has_issues() {
    let issue = ClusterIssue::builder()
      .code(ClusterIssueCode::Ec2SubnetNotFound)
      .message("Subnet not found")
      .resource_ids("subnet-12345")
      .build();

    let cluster = Cluster::builder()
      .health(ClusterHealth::builder().issues(issue).build())
      .build();

    let result = cluster_health(&cluster).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].code, "Ec2SubnetNotFound");
    assert_eq!(result[0].message, "Subnet not found");
    assert_eq!(result[0].resource_ids, vec!["subnet-12345"]);
  }

  // ---------- addon_health ----------

  #[test]
  fn addon_health_no_issues() {
    let addon = Addon::builder()
      .addon_name("vpc-cni")
      .health(AddonHealth::builder().build())
      .build();

    let result = addon_health(&[addon]).unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn addon_health_has_issues() {
    let issue = AddonIssue::builder()
      .code(AddonIssueCode::AccessDenied)
      .message("Access denied")
      .resource_ids("arn:aws:iam::123456789012:role/test")
      .build();

    let addon = Addon::builder()
      .addon_name("vpc-cni")
      .health(AddonHealth::builder().issues(issue).build())
      .build();

    let result = addon_health(&[addon]).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "vpc-cni");
    assert_eq!(result[0].code, "AccessDenied");
    assert_eq!(result[0].message, "Access denied");
    assert_eq!(
      result[0].resource_ids,
      vec!["arn:aws:iam::123456789012:role/test"]
    );
  }

  #[test]
  fn addon_health_none_health() {
    let addon = Addon::builder().addon_name("coredns").build();

    let result = addon_health(&[addon]).unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn addon_health_empty_addons() {
    let result = addon_health(&[]).unwrap();
    assert!(result.is_empty());
  }

  // ---------- eks_managed_nodegroup_health ----------

  #[test]
  fn nodegroup_health_no_issues() {
    let ng = Nodegroup::builder()
      .nodegroup_name("test-ng")
      .health(NodegroupHealth::builder().build())
      .build();

    let result = eks_managed_nodegroup_health(&[ng]).unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn nodegroup_health_has_issues() {
    let issue = Issue::builder()
      .code(NodegroupIssueCode::AccessDenied)
      .message("Access denied to node group")
      .build();

    let ng = Nodegroup::builder()
      .nodegroup_name("test-ng")
      .health(NodegroupHealth::builder().issues(issue).build())
      .build();

    let result = eks_managed_nodegroup_health(&[ng]).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "test-ng");
    assert_eq!(result[0].code, "AccessDenied");
    assert_eq!(result[0].message, "Access denied to node group");
  }

  #[test]
  fn nodegroup_health_none_health() {
    let ng = Nodegroup::builder().nodegroup_name("test-ng").build();

    let result = eks_managed_nodegroup_health(&[ng]).unwrap();
    assert!(result.is_empty());
  }

  // ---------- al2_ami_deprecation ----------

  #[test]
  fn al2_ami_deprecation_target_below_32() {
    let ng = Nodegroup::builder()
      .nodegroup_name("test-ng")
      .ami_type(AmiTypes::Al2X8664)
      .build();

    let result = al2_ami_deprecation(&[ng], 31).unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn al2_ami_deprecation_target_32_recommended() {
    let ng = Nodegroup::builder()
      .nodegroup_name("test-ng")
      .ami_type(AmiTypes::Al2X8664)
      .build();

    let result = al2_ami_deprecation(&[ng], 32).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "test-ng");
    assert_eq!(result[0].ami_type, "AL2_x86_64");
    assert!(matches!(
      result[0].finding.remediation,
      Remediation::Recommended
    ));
  }

  #[test]
  fn al2_ami_deprecation_target_33_required() {
    let ng = Nodegroup::builder()
      .nodegroup_name("test-ng")
      .ami_type(AmiTypes::Al2X8664)
      .build();

    let result = al2_ami_deprecation(&[ng], 33).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "test-ng");
    assert!(matches!(
      result[0].finding.remediation,
      Remediation::Required
    ));
  }

  #[test]
  fn al2_ami_deprecation_non_al2_ami_type() {
    let ng = Nodegroup::builder()
      .nodegroup_name("test-ng")
      .ami_type(AmiTypes::Al2023X8664Standard)
      .build();

    let result = al2_ami_deprecation(&[ng], 33).unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn al2_ami_deprecation_no_ami_type() {
    let ng = Nodegroup::builder().nodegroup_name("test-ng").build();

    let result = al2_ami_deprecation(&[ng], 33).unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn al2_ami_deprecation_mixed_ami_types() {
    let al2_ng = Nodegroup::builder()
      .nodegroup_name("al2-ng")
      .ami_type(AmiTypes::Al2X8664)
      .build();

    let al2_arm_ng = Nodegroup::builder()
      .nodegroup_name("al2-arm-ng")
      .ami_type(AmiTypes::Al2Arm64)
      .build();

    let al2023_ng = Nodegroup::builder()
      .nodegroup_name("al2023-ng")
      .ami_type(AmiTypes::Al2023X8664Standard)
      .build();

    let bottlerocket_ng = Nodegroup::builder()
      .nodegroup_name("br-ng")
      .ami_type(AmiTypes::BottlerocketX8664)
      .build();

    let result =
      al2_ami_deprecation(&[al2_ng, al2_arm_ng, al2023_ng, bottlerocket_ng], 32).unwrap();
    // Only the two AL2 nodegroups should produce findings
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "al2-ng");
    assert_eq!(result[1].name, "al2-arm-ng");
    assert!(result
      .iter()
      .all(|f| matches!(f.finding.remediation, Remediation::Recommended)));
  }

  use crate::eks::resources::VpcSubnet;

  // ---------- control_plane_ips ----------

  #[test]
  fn control_plane_ips_empty_subnets() {
    let result = control_plane_ips(&[]);
    assert!(result.is_empty());
  }

  #[test]
  fn control_plane_ips_two_azs_sufficient() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 10, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-2".into(), available_ips: 8, availability_zone_id: "use1-az2".into() },
    ];
    let result = control_plane_ips(&subnets);
    assert!(result.is_empty(), "2 AZs with >= 5 IPs should produce no findings");
  }

  #[test]
  fn control_plane_ips_one_az_insufficient() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 10, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-2".into(), available_ips: 3, availability_zone_id: "use1-az2".into() },
    ];
    let result = control_plane_ips(&subnets);
    assert!(!result.is_empty(), "only 1 AZ with >= 5 IPs should produce findings");
    assert!(result.iter().all(|f| matches!(f.finding.remediation, Remediation::Required)));
  }

  #[test]
  fn control_plane_ips_boundary_exactly_5() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 5, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-2".into(), available_ips: 5, availability_zone_id: "use1-az2".into() },
    ];
    let result = control_plane_ips(&subnets);
    assert!(result.is_empty(), "exactly 5 IPs in 2 AZs should pass");
  }

  #[test]
  fn control_plane_ips_aggregates_across_subnets_in_same_az() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1a".into(), available_ips: 3, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-1b".into(), available_ips: 3, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-2".into(), available_ips: 6, availability_zone_id: "use1-az2".into() },
    ];
    let result = control_plane_ips(&subnets);
    assert!(result.is_empty(), "3+3=6 in az1 and 6 in az2 should pass");
  }

  // ---------- pod_ips ----------

  #[test]
  fn pod_ips_empty_subnets() {
    let result = pod_ips(&[], 16, 256);
    assert!(result.is_empty(), "no subnets means no custom networking, no findings");
  }

  #[test]
  fn pod_ips_above_recommended() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 200, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-2".into(), available_ips: 100, availability_zone_id: "use1-az2".into() },
    ];
    let result = pod_ips(&subnets, 16, 256);
    assert!(result.is_empty(), "300 IPs >= 256 recommended threshold");
  }

  #[test]
  fn pod_ips_between_required_and_recommended() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 100, availability_zone_id: "use1-az1".into() },
    ];
    let result = pod_ips(&subnets, 16, 256);
    assert!(!result.is_empty());
    assert!(result.iter().all(|f| matches!(f.finding.remediation, Remediation::Recommended)));
  }

  #[test]
  fn pod_ips_below_required() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 10, availability_zone_id: "use1-az1".into() },
    ];
    let result = pod_ips(&subnets, 16, 256);
    assert!(!result.is_empty());
    assert!(result.iter().all(|f| matches!(f.finding.remediation, Remediation::Required)));
  }

  // ---------- data_plane_ips ----------

  #[test]
  fn data_plane_ips_empty_subnets() {
    let result = data_plane_ips(&[], 30, 100);
    assert!(result.is_empty());
  }

  #[test]
  fn data_plane_ips_above_recommended() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 80, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-2".into(), available_ips: 80, availability_zone_id: "use1-az2".into() },
    ];
    let result = data_plane_ips(&subnets, 30, 100);
    assert!(result.is_empty());
  }

  #[test]
  fn data_plane_ips_between_required_and_recommended() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 40, availability_zone_id: "use1-az1".into() },
    ];
    let result = data_plane_ips(&subnets, 30, 100);
    assert!(!result.is_empty());
    assert!(result.iter().all(|f| matches!(f.finding.remediation, Remediation::Recommended)));
  }

  #[test]
  fn data_plane_ips_below_required() {
    let subnets = vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 10, availability_zone_id: "use1-az1".into() },
    ];
    let result = data_plane_ips(&subnets, 30, 100);
    assert!(!result.is_empty());
    assert!(result.iter().all(|f| matches!(f.finding.remediation, Remediation::Required)));
  }

  // ---------- addon_version_compatibility ----------

  use std::collections::{HashMap as StdHashMap, HashSet};
  use crate::eks::resources::AddonVersion;

  #[test]
  fn addon_version_compat_all_supported() {
    let addon = Addon::builder()
      .addon_name("vpc-cni")
      .addon_version("v1.15.0")
      .build();

    let current = StdHashMap::from([("vpc-cni".into(), AddonVersion {
      latest: "v1.15.0".into(),
      default: "v1.14.0".into(),
      supported_versions: HashSet::from(["v1.15.0".into(), "v1.14.0".into()]),
    })]);
    let target = StdHashMap::from([("vpc-cni".into(), AddonVersion {
      latest: "v1.16.0".into(),
      default: "v1.15.0".into(),
      supported_versions: HashSet::from(["v1.16.0".into(), "v1.15.0".into()]),
    })]);

    let result = addon_version_compatibility(&[addon], &current, &target);
    assert!(result.is_empty(), "version supported in both should produce no findings");
  }

  #[test]
  fn addon_version_compat_not_latest_recommended() {
    let addon = Addon::builder()
      .addon_name("vpc-cni")
      .addon_version("v1.14.0")
      .build();

    let current = StdHashMap::from([("vpc-cni".into(), AddonVersion {
      latest: "v1.15.0".into(),
      default: "v1.14.0".into(),
      supported_versions: HashSet::from(["v1.15.0".into(), "v1.14.0".into()]),
    })]);
    let target = StdHashMap::from([("vpc-cni".into(), AddonVersion {
      latest: "v1.16.0".into(),
      default: "v1.15.0".into(),
      supported_versions: HashSet::from(["v1.16.0".into(), "v1.15.0".into(), "v1.14.0".into()]),
    })]);

    let result = addon_version_compatibility(&[addon], &current, &target);
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Recommended));
  }

  #[test]
  fn addon_version_compat_unsupported_on_target_required() {
    let addon = Addon::builder()
      .addon_name("vpc-cni")
      .addon_version("v1.12.0")
      .build();

    let current = StdHashMap::from([("vpc-cni".into(), AddonVersion {
      latest: "v1.15.0".into(),
      default: "v1.14.0".into(),
      supported_versions: HashSet::from(["v1.15.0".into(), "v1.14.0".into(), "v1.12.0".into()]),
    })]);
    let target = StdHashMap::from([("vpc-cni".into(), AddonVersion {
      latest: "v1.16.0".into(),
      default: "v1.15.0".into(),
      supported_versions: HashSet::from(["v1.16.0".into(), "v1.15.0".into()]),
    })]);

    let result = addon_version_compatibility(&[addon], &current, &target);
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Required));
  }

  // ---------- eks_managed_nodegroup_update ----------

  use aws_sdk_eks::types::{AutoScalingGroup as EksAutoScalingGroup, NodegroupResources};
  use crate::eks::resources::LaunchTemplate;

  #[test]
  fn mng_update_no_launch_template() {
    let ng = Nodegroup::builder().nodegroup_name("test").build();
    let result = eks_managed_nodegroup_update(&ng, None);
    assert!(result.is_empty());
  }

  #[test]
  fn mng_update_current_equals_latest() {
    let ng = Nodegroup::builder()
      .nodegroup_name("test")
      .resources(
        NodegroupResources::builder()
          .auto_scaling_groups(
            EksAutoScalingGroup::builder().name("asg-1").build()
          )
          .build()
      )
      .build();
    let lt = LaunchTemplate {
      name: "lt-1".into(),
      id: "lt-abc".into(),
      current_version: "3".into(),
      latest_version: "3".into(),
    };
    let result = eks_managed_nodegroup_update(&ng, Some(&lt));
    assert!(result.is_empty(), "current == latest should produce no findings");
  }

  #[test]
  fn mng_update_current_behind_latest() {
    let ng = Nodegroup::builder()
      .nodegroup_name("test")
      .resources(
        NodegroupResources::builder()
          .auto_scaling_groups(
            EksAutoScalingGroup::builder().name("asg-1").build()
          )
          .build()
      )
      .build();
    let lt = LaunchTemplate {
      name: "lt-1".into(),
      id: "lt-abc".into(),
      current_version: "2".into(),
      latest_version: "5".into(),
    };
    let result = eks_managed_nodegroup_update(&ng, Some(&lt));
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Recommended));
  }

  // ---------- self_managed_nodegroup_update ----------

  #[test]
  fn smng_update_current_equals_latest() {
    let asg = AutoScalingGroup::builder()
      .auto_scaling_group_name("asg-1")
      .build();
    let lt = LaunchTemplate {
      name: "lt-1".into(),
      id: "lt-abc".into(),
      current_version: "3".into(),
      latest_version: "3".into(),
    };
    let result = self_managed_nodegroup_update(&asg, &lt);
    assert!(result.is_none());
  }

  #[test]
  fn smng_update_current_behind_latest() {
    let asg = AutoScalingGroup::builder()
      .auto_scaling_group_name("asg-1")
      .build();
    let lt = LaunchTemplate {
      name: "lt-1".into(),
      id: "lt-abc".into(),
      current_version: "1".into(),
      latest_version: "3".into(),
    };
    let result = self_managed_nodegroup_update(&asg, &lt);
    assert!(result.is_some());
    assert!(matches!(result.unwrap().finding.remediation, Remediation::Recommended));
  }
}
