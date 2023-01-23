// use k8s_openapi::api::{apps, batch, core, policy};
use std::collections::BTreeMap;

use k8s_openapi::api::core;
use kube::{api::Api, Client, CustomResource};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
  finding::{self, Findings},
  version,
};

/// Node details as viewed from the Kubernetes API
///
/// Contains information related to the Kubernetes component versions
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct NodeFinding {
  pub(crate) name: String,
  pub(crate) kubelet_version: String,
  pub(crate) kubernetes_version: String,
  pub(crate) control_plane_version: String,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

impl Findings for Vec<NodeFinding> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return None;
    }
    let mut counts: BTreeMap<(String, String), isize> = BTreeMap::new();
    for node in self {
      *counts
        .entry((
          node.kubernetes_version.to_owned(),
          node.control_plane_version.to_owned(),
        ))
        .or_insert(0) += 1
    }

    let mut summary = String::new();
    summary.push_str(&format!("{leading_whitespace}| Nodes | Kubelet Version | Control Plane Version |\n"));
    summary.push_str(&format!("{leading_whitespace}| :---: | :-------------- | :-------------------- |\n"));

    for (k, v) in counts.iter() {
      summary.push_str(&format!("{leading_whitespace}| {v} | v{} | v{} |\n", k.0, k.1));
    }

    let mut table = String::new();
    table.push_str(&format!("{leading_whitespace}|       | Name  | Kubelet Version | Control Plane Version |\n"));
    table.push_str(&format!("{leading_whitespace}| :---: | :---- | :-------------- | :-------------------- |\n"));

    for finding in self {
      table.push_str(&format!(
        "{}| {} | {} | v{} | v{} |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.name,
        finding.kubernetes_version,
        finding.control_plane_version,
      ))
    }

    Some(format!("{summary}\n{table}\n"))
  }
}

/// Returns all of the nodes in the cluster
pub(crate) async fn version_skew(client: &Client, cluster_version: &str) -> Result<Vec<NodeFinding>, anyhow::Error> {
  let api: Api<core::v1::Node> = Api::all(client.clone());
  let node_list = api.list(&Default::default()).await?;

  let mut findings = vec![];

  for node in &node_list {
    let status = node.status.as_ref().unwrap();
    let node_info = status.node_info.as_ref().unwrap();
    let kubelet_version = node_info.kubelet_version.to_owned();

    let node_minor_version = version::parse_minor(&kubelet_version).unwrap();
    let control_plane_minor_version = version::parse_minor(cluster_version)?;
    let version_skew = control_plane_minor_version - node_minor_version;
    if version_skew == 0 {
      continue;
    }

    // Prior to upgrade, the node version should not be more than 1 version behind
    // the control plane version. If it is, the node must be upgraded before
    // attempting the cluster upgrade
    let remediation = match version_skew {
      1 => finding::Remediation::Recommended,
      _ => finding::Remediation::Required,
    };

    let node = NodeFinding {
      name: node.metadata.name.as_ref().unwrap().to_owned(),
      kubelet_version: kubelet_version.to_owned(),
      kubernetes_version: version::normalize(&kubelet_version).unwrap(),
      control_plane_version: cluster_version.to_owned(),
      remediation,
      fcode: finding::Code::K8S001,
    };

    findings.push(node)
  }

  Ok(findings)
}

/// Custom resource definition for ENIConfig as specified in the AWS VPC CNI
///
/// This makes it possible to query the custom resources in the cluster
/// for extracting information from the ENIConfigs (if present)
/// <https://github.com/aws/amazon-vpc-cni-k8s/blob/master/charts/aws-vpc-cni/crds/customresourcedefinition.yaml>
#[derive(Clone, CustomResource, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize)]
#[kube(
  derive = "Default",
  derive = "PartialEq",
  group = "crd.k8s.amazonaws.com",
  kind = "ENIConfig",
  schema = "derived",
  plural = "eniconfigs",
  singular = "eniconfig",
  version = "v1alpha1"
)]
pub(crate) struct EniConfigSpec {
  pub(crate) subnet: Option<String>,
  pub(crate) security_groups: Option<Vec<String>>,
}

/// Returns all of the ENIConfigs in the cluster, if any are present
///
/// This is used to extract the subnet ID(s) to retrieve the number of
/// available IPs in the subnet(s) when custom networking is enabled
pub(crate) async fn get_eniconfigs(client: &Client) -> Result<Vec<ENIConfig>, anyhow::Error> {
  let api = Api::<ENIConfig>::all(client.clone());
  let eniconfigs: Vec<ENIConfig> = api.list(&Default::default()).await?.items;

  Ok(eniconfigs)
}

// async fn _get_podsecuritypolicies(
//   client: &Client,
// ) -> Result<Vec<policy::v1beta1::PodSecurityPolicy>, anyhow::Error> {
//   let api: Api<policy::v1beta1::PodSecurityPolicy> = Api::all(client.clone());
//   let nodes = api.list(&Default::default()).await?;

//   Ok(nodes.items)
// }

// async fn _get_cronjobs(client: &Client) -> Result<Vec<batch::v1::CronJob>, anyhow::Error> {
//   let api: Api<batch::v1::CronJob> = Api::all(client.clone());
//   let cronjobs = api.list(&Default::default()).await?;

//   Ok(cronjobs.items)
// }

// async fn _get_daemonset(client: &Client) -> Result<Vec<apps::v1::DaemonSet>, anyhow::Error> {
//   let api: Api<apps::v1::DaemonSet> = Api::all(client.clone());
//   let daemonsets = api.list(&Default::default()).await?;

//   Ok(daemonsets.items)
// }

// async fn _get_deployments(client: &Client) -> Result<Vec<apps::v1::Deployment>, anyhow::Error> {
//   let api: Api<apps::v1::Deployment> = Api::all(client.clone());
//   let deployments = api.list(&Default::default()).await?;

//   Ok(deployments.items)
// }

// async fn _get_jobs(client: &Client) -> Result<Vec<batch::v1::Job>, anyhow::Error> {
//   let api: Api<batch::v1::Job> = Api::all(client.clone());
//   let jobs = api.list(&Default::default()).await?;

//   Ok(jobs.items)
// }

// async fn _get_replicasets(client: &Client) -> Result<Vec<apps::v1::ReplicaSet>, anyhow::Error> {
//   let api: Api<apps::v1::ReplicaSet> = Api::all(client.clone());
//   let replicasets = api.list(&Default::default()).await?;

//   Ok(replicasets.items)
// }

// async fn _get_statefulsets(client: &Client) -> Result<Vec<apps::v1::StatefulSet>, anyhow::Error> {
//   let api: Api<apps::v1::StatefulSet> = Api::all(client.clone());
//   let statefulsets = api.list(&Default::default()).await?;

//   Ok(statefulsets.items)
// }
