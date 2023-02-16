use std::collections::HashMap;

use anyhow::Result;
use k8s_openapi::api::core;
use kube::{api::Api, Client};
use serde::{Deserialize, Serialize};
use tabled::{locator::ByColumnName, Disable, Margin, Style, Table, Tabled};

use crate::{
  finding::{self, Findings},
  k8s::resources::Resource,
  version,
};

/// Node details as viewed from the Kubernetes API
///
/// Contains information related to the Kubernetes component versions
#[derive(Clone, Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct VersionSkew {
  #[tabled(inline)]
  pub finding: finding::Finding,
  pub name: String,
  #[tabled(skip)]
  pub kubelet_version: String,
  #[tabled(rename = "NODE")]
  pub kubernetes_version: String,
  #[tabled(rename = "CONTROL PLANE")]
  pub control_plane_version: String,
  #[tabled(rename = "SKEW")]
  pub version_skew: String,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct VersionSkewSummary {
  #[tabled(inline)]
  pub version_skew: VersionSkew,
  pub quantity: i32,
}

impl Findings for Vec<VersionSkew> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - No reported findings regarding version skew between the control plane and nodes"
      ));
    }

    let mut summary: HashMap<(String, String, String, String, String), VersionSkewSummary> = HashMap::new();
    for node in self {
      let key = (
        node.finding.code.to_string(),
        node.finding.symbol.to_owned(),
        node.finding.remediation.to_string(),
        node.kubernetes_version.to_owned(),
        node.control_plane_version.to_owned(),
      );

      if let Some(summary) = summary.get_mut(&key) {
        summary.quantity += 1;
      } else {
        summary.insert(
          key,
          VersionSkewSummary {
            version_skew: node.clone(),
            quantity: 1,
          },
        );
      }
    }

    let mut summary_tbl = Table::new(summary);
    summary_tbl
      .with(Margin::new(1, 0, 0, 0).set_fill('\t', 'x', 'x', 'x'))
      .with(Disable::column(ByColumnName::new("String")))
      .with(Disable::column(ByColumnName::new("NAME")))
      .with(Style::markdown());

    let mut table = Table::new(self);
    table
      .with(Disable::column(ByColumnName::new("CHECK")))
      .with(Margin::new(1, 0, 0, 0).set_fill('\t', 'x', 'x', 'x'))
      .with(Style::markdown());

    Ok(format!("{summary_tbl}\n\n{table}\n"))
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

/// Returns all of the nodes in the cluster
pub async fn version_skew(client: &Client, cluster_version: &str) -> Result<Vec<VersionSkew>> {
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

    let finding = finding::Finding {
      code: finding::Code::K8S001,
      symbol: remediation.symbol(),
      remediation,
    };

    let node = VersionSkew {
      finding,
      name: node.metadata.name.as_ref().unwrap().to_owned(),
      kubelet_version: kubelet_version.to_owned(),
      kubernetes_version: format!("v{}", version::normalize(&kubelet_version).unwrap()),
      control_plane_version: format!("v{}", cluster_version.to_owned()),
      version_skew: format!("+{}", version_skew),
    };

    findings.push(node)
  }

  Ok(findings)
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct MinReplicas {
  #[tabled(inline)]
  pub finding: finding::Finding,
  #[tabled(inline)]
  pub resource: Resource,
  /// Number of replicas
  pub replicas: i32,
}

impl Findings for Vec<MinReplicas> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - All relevant Kubernetes workloads have at least 3 replicas specified"
      ));
    }

    let mut table = Table::new(self);
    table
      .with(Disable::column(ByColumnName::new("CHECK")))
      .with(Margin::new(1, 0, 0, 0).set_fill('\t', 'x', 'x', 'x'))
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

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct MinReadySeconds {
  #[tabled(inline)]
  pub finding: finding::Finding,
  #[tabled(inline)]
  pub resource: Resource,
  /// Min ready seconds
  pub seconds: i32,
}

impl Findings for Vec<MinReadySeconds> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - All relevant Kubernetes workloads minReadySeconds set to more than 0"
      ));
    }

    let mut table = Table::new(self);
    table
      .with(Disable::column(ByColumnName::new("CHECK")))
      .with(Margin::new(1, 0, 0, 0).set_fill('\t', 'x', 'x', 'x'))
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

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct Probe {
  #[tabled(inline)]
  pub finding: finding::Finding,

  #[tabled(inline)]
  pub(crate) resource: Resource,
}

impl Findings for Vec<Probe> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - All relevant Kubernetes workloads have a readiness probe configured"
      ));
    }

    let mut table = Table::new(self);
    table
      .with(Disable::column(ByColumnName::new("CHECK")))
      .with(Margin::new(1, 0, 0, 0).set_fill('\t', 'x', 'x', 'x'))
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
  /// K8S006 - check if resources have readinessProbe
  fn readiness_probe(&self) -> Option<Probe>;
  // /// K8S008 - check if resources use the Docker socket
  // fn docker_socket(&self) -> Result<Option<DockerSocketFinding>>;
}
