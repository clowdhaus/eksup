use aws_sdk_eks::model::Cluster;
use k8s_openapi::api::core::v1::NodeSystemInfo;
use std::collections::BTreeMap;

pub async fn execute(cluster: &Cluster, nodes: &Vec<NodeSystemInfo>) -> Result<(), anyhow::Error> {
  version_skew(cluster.version.as_ref().unwrap(), nodes).await?;

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

/// Check if there are any nodes that are not at the same minor version as the control plane
///
/// Report on the nodes that do not match the same minor version as the control plane
/// so that users can remediate before upgrading.
///
/// TODO - how to make check results consistent and not one-offs? Needs to align with
/// the goal of multiple return types (JSON, CSV, etc.)
async fn version_skew(
  control_plane_version: &str,
  nodes: &Vec<NodeSystemInfo>,
) -> Result<(), anyhow::Error> {
  let cp_minor = parse_minor_version(control_plane_version)?;
  let mut node_versions: BTreeMap<String, isize> = BTreeMap::new();

  for node in nodes {
    *node_versions
      .entry(node.kubelet_version.clone())
      .or_insert(0) += 1;
  }

  for (key, value) in node_versions.iter() {
    let minor = parse_minor_version(key)?;
    if minor != cp_minor {
      let version = normalize_version(key)?;
      println!("There are {value} nodes that are at version v{version} which do not match the control plane version v{control_plane_version}");
    }
  }

  Ok(())
}

fn _control_plane_ips() -> Result<Vec<String>, anyhow::Error> {
  todo!()
}
