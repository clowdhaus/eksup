use std::collections::BTreeMap;

use anyhow::Result;
use k8s_openapi::api::core;
use kube::{api::Api, Client};
use serde::{Deserialize, Serialize};
use tabled::{format::Format, object::Rows, Modify, Style, Table, Tabled};

use crate::{
  finding::{self, Findings},
  k8s::resources::Resource,
  version,
};

/// Node details as viewed from the Kubernetes API
///
/// Contains information related to the Kubernetes component versions
#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct NodeFinding {
  #[tabled(rename = "CHECK")]
  pub fcode: finding::Code,
  pub remediation: finding::Remediation,
  pub name: String,
  pub kubelet_version: String,
  pub kubernetes_version: String,
  pub control_plane_version: String,
}

impl Findings for Vec<NodeFinding> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}✅ - No reported findings regarding version skew between the control plane and nodes"
      ));
    }
    let mut counts: BTreeMap<(String, String, String), isize> = BTreeMap::new();
    for node in self {
      *counts
        .entry((
          node.remediation.symbol().to_owned(),
          node.kubernetes_version.to_owned(),
          node.control_plane_version.to_owned(),
        ))
        .or_insert(0) += 1
    }

    let mut summary = String::new();
    summary.push_str(&format!(
      "{leading_whitespace}|  -  | Nodes | Kubelet Version | Control Plane Version |\n"
    ));
    summary.push_str(&format!(
      "{leading_whitespace}| :---: | :---: | :-------------- | :-------------------- |\n"
    ));

    for (k, v) in counts.iter() {
      summary.push_str(&format!(
        "{leading_whitespace}| {sym} | {v} | `v{kube}` | `v{cp}` |\n",
        sym = k.0,
        kube = k.1,
        cp = k.2
      ));
    }

    let mut table = String::new();
    table.push_str(&format!(
      "{leading_whitespace}|   -   | Node Name | Kubelet Version | Control Plane Version |\n"
    ));
    table.push_str(&format!(
      "{leading_whitespace}| :---: | :-------- | :-------------- | :-------------------- |\n"
    ));

    for finding in self {
      table.push_str(&format!(
        "{}| {} | `{}` | `v{}` | `v{}` |\n",
        leading_whitespace,
        finding.remediation.symbol(),
        finding.name,
        finding.kubernetes_version,
        finding.control_plane_version,
      ))
    }

    Some(format!("{summary}\n{table}\n"))
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

/// Returns all of the nodes in the cluster
pub async fn version_skew(client: &Client, cluster_version: &str) -> Result<Vec<NodeFinding>> {
  let api: Api<core::v1::Node> = Api::all(client.to_owned());
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

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct MinReplicas {
  #[tabled(rename = "CHECK")]
  pub fcode: finding::Code,
  pub remediation: finding::Remediation,
  #[tabled(inline)]
  pub resource: Resource,
  /// Number of replicas
  pub replicas: i32,
}

impl Findings for Vec<MinReplicas> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}✅ - All relevant Kubernetes workloads have at least 3 replicas specified"
      ));
    }

    let mut table = String::new();
    table.push_str(&format!(
      "{leading_whitespace}|  -  | Name | Namespace | Kind | Minimum Replicas |\n"
    ));
    table.push_str(&format!(
      "{leading_whitespace}| :---: | :--- | :------ | :--- | :--------------- |\n"
    ));

    for finding in self {
      table.push_str(&format!(
        "{leading_whitespace}| {} | {} | {} | {} | {} |\n",
        finding.remediation.symbol(),
        finding.resource.name,
        finding.resource.namespace,
        finding.resource.kind,
        finding.replicas,
      ))
    }

    Some(format!("{table}\n"))
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

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct MinReadySeconds {
  #[tabled(rename = "CHECK")]
  pub fcode: finding::Code,
  pub remediation: finding::Remediation,
  #[tabled(inline)]
  pub resource: Resource,
  /// Min ready seconds
  pub seconds: i32,
}

impl Findings for Vec<MinReadySeconds> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Option<String> {
    if self.is_empty() {
      return Some(format!(
        "{leading_whitespace}✅ - All relevant Kubernetes workloads minReadySeconds set to more than 0"
      ));
    }

    let mut table = String::new();
    table.push_str(&format!(
      "{leading_whitespace}|  -  | Name | Namespace | Kind | minReadySeconds |\n"
    ));
    table.push_str(&format!(
      "{leading_whitespace}| :---: | :--- | :------ | :--- | :--------------- |\n"
    ));

    for finding in self {
      table.push_str(&format!(
        "{leading_whitespace}| {} | {} | {} | {} | {} |\n",
        finding.remediation.symbol(),
        finding.resource.name,
        finding.resource.namespace,
        finding.resource.kind,
        finding.seconds,
      ))
    }

    Some(format!("{table}\n"))
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

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct PodDisruptionBudgetFinding {
  pub(crate) resource: Resource,
  /// Has pod associated pod disruption budget
  /// TODO - more relevant information than just present?
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct PodTopologyDistributionFinding {
  pub(crate) resource: Resource,
  ///
  pub(crate) anti_affinity: Option<String>,
  ///
  pub(crate) toplogy_spread_constraints: Option<String>,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ProbeFinding {
  pub(crate) resource: Resource,
  ///
  pub(crate) readiness: Option<String>,
  pub(crate) liveness: Option<String>,
  pub(crate) startup: Option<String>,

  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TerminationGracePeriodFinding {
  pub(crate) resource: Resource,
  /// Min ready seconds
  pub(crate) seconds: i32,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct DockerSocketFinding {
  pub(crate) resource: Resource,
  ///
  pub(crate) volumes: Vec<String>,
  pub(crate) remediation: finding::Remediation,
  pub(crate) fcode: finding::Code,
}

pub trait K8sFindings {
  fn get_resource(&self) -> Resource;
  /// K8S002 - check if resources contain a minimum of 3 replicas
  fn min_replicas(&self) -> Option<MinReplicas>;
  /// K8S003 - check if resources contain minReadySeconds > 0
  fn min_ready_seconds(&self) -> Option<MinReadySeconds>;
  // /// K8S004 - check if resources have associated podDisruptionBudgets
  // fn pod_disruption_budget(&self) -> Result<Option<PodDisruptionBudgetFinding>>;
  // /// K8S005 - check if resources have podAntiAffinity or topologySpreadConstraints
  // fn pod_topology_distribution(&self) -> Result<Option<PodTopologyDistributionFinding>>;
  // /// K8S006 - check if resources have readinessProbe
  // fn readiness_probe(&self) -> Result<Option<ProbeFinding>>;
  // /// K8S008 - check if resources use the Docker socket
  // fn docker_socket(&self) -> Result<Option<DockerSocketFinding>>;
}
