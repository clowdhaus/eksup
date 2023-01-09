use super::aws;

use aws_sdk_autoscaling::model::AutoScalingGroup;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_eks::model::{Cluster, FargateProfile, Nodegroup, NodegroupIssueCode};
use k8s_openapi::api::core::v1::Node;
use std::collections::HashSet;

pub async fn execute(
  aws_shared_config: &aws_config::SdkConfig,
  cluster: &Cluster,
  nodes: &Vec<Node>,
) -> Result<(), anyhow::Error> {
  // Construct clients once
  let asg_client = aws_sdk_autoscaling::Client::new(aws_shared_config);
  let ec2_client = aws_sdk_ec2::Client::new(aws_shared_config);
  let eks_client = aws_sdk_eks::Client::new(aws_shared_config);

  let cluster_name = cluster.name.as_ref().unwrap();

  // Get data plane components once
  let eks_managed_node_groups = aws::get_eks_managed_node_groups(&eks_client, cluster_name).await?;
  let self_managed_node_groups =
    aws::get_self_managed_node_groups(&asg_client, cluster_name).await?;
  let fargate_profiles = aws::get_fargate_profiles(&eks_client, cluster_name).await?;

  // Checks
  version_skew(cluster.version.as_ref().unwrap(), nodes).await?;
  ips_available_for_control_plane(cluster, &ec2_client).await?;
  ips_available_for_data_plane(
    &ec2_client,
    eks_managed_node_groups.clone(),
    fargate_profiles.clone(),
    self_managed_node_groups.clone(),
  )
  .await?;

  if let Some(eks_managed_node_groups) = eks_managed_node_groups {
    eks_managed_node_group_health(eks_managed_node_groups).await?;
  }

  Ok(())
}

/// Given a version, parse the minor version
///
/// For example, the format Amazon EKS of v1.20.7-eks-123456 returns 20
/// Or the format of v1.22.7 returns 22
fn parse_minor_version(version: &str) -> Result<u32, anyhow::Error> {
  let version = version.split('.').collect::<Vec<&str>>();
  let minor_version = version[1].parse::<u32>()?;

  Ok(minor_version)
}

/// Given a version, normalize to a consistent format
///
/// For example, the format Amazon EKS uses is v1.20.7-eks-123456 which is normalized to 1.20
fn normalize_version(version: &str) -> Result<String, anyhow::Error> {
  let version = version.split('.').collect::<Vec<&str>>();
  let normalized_version = format!("{}.{}", version[0].replace('v', ""), version[1]);

  Ok(normalized_version)
}

#[allow(dead_code)]
#[derive(Debug)]
struct NodeDetail {
  name: String,
  container_runtime: String,
  kernel_version: String,
  kube_proxy_version: String,
  kublet_version: String,
  kubernetes_version: String,
  control_plane_version: String,
}

/// Check if there are any nodes that are not at the same minor version as the control plane
///
/// Report on the nodes that do not match the same minor version as the control plane
/// so that users can remediate before upgrading.
///
/// TODO - how to make check results consistent and not one-offs? Needs to align with
/// the goal of multiple return types (JSON, CSV, etc.)
async fn version_skew(
  control_plane_version: &str,
  nodes: &Vec<Node>,
) -> Result<Option<Vec<NodeDetail>>, anyhow::Error> {
  let control_plane_minor_version = parse_minor_version(control_plane_version)?;

  let mut skewed: Vec<NodeDetail> = Vec::new();

  for node in nodes {
    let status = node.status.as_ref().unwrap();
    let node_info = status.node_info.as_ref().unwrap();
    let kubelet_version = node_info.kubelet_version.to_owned();

    let node_minor_version = parse_minor_version(&kubelet_version)?;
    if control_plane_minor_version != node_minor_version {
      let node_detail = NodeDetail {
        name: node.metadata.name.as_ref().unwrap().to_owned(),
        container_runtime: node_info.container_runtime_version.to_owned(),
        kernel_version: node_info.kernel_version.to_owned(),
        kube_proxy_version: node_info.kube_proxy_version.to_owned(),
        kublet_version: kubelet_version.to_owned(),
        kubernetes_version: normalize_version(&kubelet_version)?,
        control_plane_version: control_plane_version.to_owned(),
      };
      skewed.push(node_detail);
    }
  }

  if skewed.is_empty() {
    return Ok(None);
  }

  println!("Skewed node versions: {skewed:#?}");

  Ok(Some(skewed))
}

/// Data of significance with regards to subnets and cluster upgrade
#[allow(dead_code)]
#[derive(Debug)]
struct Subnet {
  id: String,
  availability_zone: String,
  availability_zone_id: String,
  available_ips: i32,
  cidr_block: String,
}

/// Reports IPs by subnet for the data plane
async fn ips_available_for_control_plane(
  cluster: &Cluster,
  client: &aws_sdk_ec2::Client,
) -> Result<Vec<Subnet>, anyhow::Error> {
  let subnet_ids = cluster
    .resources_vpc_config()
    .unwrap()
    .subnet_ids
    .as_ref()
    .unwrap();

  let aws_subnets = aws::get_subnets(client, subnet_ids.clone()).await?;
  let mut subnets: Vec<Subnet> = Vec::new();

  for subnet in aws_subnets.iter() {
    let id = subnet.subnet_id.as_ref().unwrap();

    subnets.push(Subnet {
      id: id.to_owned(),
      availability_zone: subnet.availability_zone.as_ref().unwrap().to_owned(),
      availability_zone_id: subnet.availability_zone_id.as_ref().unwrap().to_owned(),
      available_ips: subnet.available_ip_address_count.unwrap(),
      cidr_block: subnet.cidr_block.as_ref().unwrap().to_owned(),
    })
  }

  println!("Conctrol plane subnets: {subnets:#?}");

  Ok(subnets)
}

/// Reports IPs by subnet for the data plane
async fn ips_available_for_data_plane(
  ec2_client: &Ec2Client,
  eks_managed_node_groups: Option<Vec<Nodegroup>>,
  fargate_profiles: Option<Vec<FargateProfile>>,
  self_managed_node_groups: Option<Vec<AutoScalingGroup>>,
) -> Result<Vec<Subnet>, anyhow::Error> {
  // Dedupe subnet IDs that are shared across different compute constructs
  let mut subnet_ids = HashSet::new();

  // EKS managed node group subnets
  if let Some(nodegroups) = eks_managed_node_groups {
    for group in nodegroups {
      let subnets = group.subnets.unwrap();
      for subnet in subnets {
        subnet_ids.insert(subnet.to_owned());
      }
    }
  }

  // Self managed node group subnets
  if let Some(nodegroups) = self_managed_node_groups {
    for group in nodegroups {
      let subnets = group.vpc_zone_identifier.unwrap();
      for subnet in subnets.split(',') {
        subnet_ids.insert(subnet.to_owned());
      }
    }
  }

  // Fargate profile subnets
  if let Some(profiles) = fargate_profiles {
    for profile in profiles {
      let subnets = profile.subnets.unwrap();
      for subnet in subnets {
        subnet_ids.insert(subnet.to_owned());
      }
    }
  }

  // Send one query of all the subnets used in the data plane
  let aws_subnets = aws::get_subnets(ec2_client, subnet_ids.into_iter().collect()).await?;
  let mut subnets: Vec<Subnet> = Vec::new();

  for subnet in aws_subnets.iter() {
    let id = subnet.subnet_id.as_ref().unwrap();

    subnets.push(Subnet {
      id: id.to_owned(),
      availability_zone: subnet.availability_zone.as_ref().unwrap().to_owned(),
      availability_zone_id: subnet.availability_zone_id.as_ref().unwrap().to_owned(),
      available_ips: subnet.available_ip_address_count.unwrap(),
      cidr_block: subnet.cidr_block.as_ref().unwrap().to_owned(),
    })
  }

  println!("Data plane subnets: {subnets:#?}");

  Ok(subnets)
}

/// Nodegroup health issue data
#[allow(dead_code)]
#[derive(Debug)]
struct NodegroupHealthIssue {
  name: String,
  code: String,
  message: String,
}

/// Check for any reported health issues on EKS managed node groups
async fn eks_managed_node_group_health(
  node_groups: Vec<Nodegroup>,
) -> Result<Option<Vec<NodegroupHealthIssue>>, anyhow::Error> {
  let mut health_issues: Vec<NodegroupHealthIssue> = Vec::new();

  for group in node_groups {
    let name = group.nodegroup_name.unwrap();
    if let Some(health) = group.health {
      if let Some(issues) = health.issues {
        for issue in issues {
          let code = issue.code().unwrap_or(&NodegroupIssueCode::InternalFailure);
          let message = issue.message().unwrap_or("");
          health_issues.push(NodegroupHealthIssue {
            name: name.to_owned(),
            code: code.as_ref().to_string(),
            message: message.to_owned(),
          });
        }
      }
    }
  }

  if health_issues.is_empty() {
    return Ok(None);
  }

  println!("Nodegroup health issues: {health_issues:#?}");

  Ok(Some(health_issues))
}
