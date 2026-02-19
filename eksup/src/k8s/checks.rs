use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tabled::{
  Table, Tabled,
  settings::{Margin, Remove, Style, location::ByColumnName},
};

use crate::{
  finding::{self, Code, Finding, Findings, Remediation},
  k8s::resources::{self, Resource},
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
      .with(Margin::new(1, 0, 0, 0).fill('\t', 'x', 'x', 'x'))
      .with(Remove::column(ByColumnName::new("String")))
      .with(Remove::column(ByColumnName::new("NAME")))
      .with(Style::markdown());

    let mut table = Table::new(self);
    table
      .with(Remove::column(ByColumnName::new("CHECK")))
      .with(Margin::new(1, 0, 0, 0).fill('\t', 'x', 'x', 'x'))
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
pub fn version_skew(nodes: &[resources::Node], cluster_version: &str) -> Result<Vec<VersionSkew>> {
  let mut findings = vec![];

  for node in nodes {
    let control_plane_minor_version = version::parse_minor(cluster_version)?;
    let version_skew = control_plane_minor_version - node.minor_version;
    if version_skew <= 0 {
      continue;
    }

    // Prior to upgrade, the node version (kubelet) should not be more than 3 version behind
    // the control plane version (api server). If it is, the node must be upgraded before
    // attempting the cluster upgrade
    let remediation = match version_skew {
      1 | 2 => Remediation::Recommended,
      _ => Remediation::Required,
    };

    let node = VersionSkew {
      finding: Finding::new(Code::K8S001, remediation),
      name: node.name.to_owned(),
      kubelet_version: node.kubelet_version.to_owned(),
      kubernetes_version: format!("v{}", version::normalize(&node.kubelet_version)?),
      control_plane_version: format!("v{cluster_version}"),
      version_skew: format!("+{version_skew}"),
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

finding::impl_findings!(MinReplicas, "✅ - All relevant Kubernetes workloads have at least 3 replicas specified");

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

finding::impl_findings!(MinReadySeconds, "✅ - All relevant Kubernetes workloads minReadySeconds set to more than 0");

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct PodTopologyDistribution {
  #[tabled(inline)]
  pub finding: finding::Finding,
  #[tabled(inline)]
  pub resource: Resource,

  pub anti_affinity: bool,
  pub topology_spread_constraints: bool,
}

finding::impl_findings!(PodTopologyDistribution, "✅ - All relevant Kubernetes workloads have either podAntiAffinity or topologySpreadConstraints set");

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct Probe {
  #[tabled(inline)]
  pub finding: finding::Finding,

  #[tabled(inline)]
  pub resource: Resource,
  #[tabled(rename = "READINESS PROBE")]
  pub readiness_probe: bool,
}

finding::impl_findings!(Probe, "✅ - All relevant Kubernetes workloads have a readiness probe configured");

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct TerminationGracePeriod {
  #[tabled(inline)]
  pub finding: finding::Finding,

  #[tabled(inline)]
  pub resource: Resource,
  /// Min ready seconds
  pub termination_grace_period: i64,
}

finding::impl_findings!(TerminationGracePeriod, "✅ - No StatefulSet workloads have a terminationGracePeriodSeconds set to more than 0");

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct DockerSocket {
  #[tabled(inline)]
  pub finding: finding::Finding,

  #[tabled(inline)]
  pub resource: Resource,

  pub docker_socket: bool,
}

finding::impl_findings!(DockerSocket, "✅ - No relevant Kubernetes workloads are found to be utilizing the Docker socket");

#[derive(Clone, Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct KubeProxyVersionSkew {
  #[tabled(inline)]
  pub finding: finding::Finding,
  #[tabled(rename = "API SERVER")]
  pub api_server_version: String,
  #[tabled(rename = "KUBE PROXY")]
  pub kube_proxy_version: String,
  #[tabled(rename = "SKEW")]
  pub version_skew: String,
}

pub fn kube_proxy_version_skew(
  resources: &[resources::StdResource],
  cluster_version: &str,
) -> Result<Vec<KubeProxyVersionSkew>> {
  let kube_proxy = match resources
    .iter()
    .find(|r| r.metadata.kind == resources::Kind::DaemonSet && r.metadata.name == "kube-proxy")
  {
    Some(k) => k,
    None => {
      tracing::warn!("Unable to find kube-proxy daemonset");
      return Ok(vec![]);
    }
  };

  let ptmpl = kube_proxy.spec.template.as_ref().context("kube-proxy has no pod template")?;
  let pspec = ptmpl.spec.as_ref().context("kube-proxy pod template has no spec")?;
  let first_container = pspec.containers.first().context("kube-proxy has no containers")?;
  let image_tag = first_container.image.as_deref()
    .and_then(|img| img.split(':').nth(1))
    .context("kube-proxy container image has no version tag")?;
  let kproxy_minor_version = version::parse_minor(image_tag)?;

  let control_plane_minor_version = version::parse_minor(cluster_version)?;
  let version_skew = control_plane_minor_version - kproxy_minor_version;
  if version_skew <= 0 {
    return Ok(vec![]);
  }

  // Prior to upgrade, kube-proxy should not be more than 3 version behind
  // the api server. If it is, kube-proxy must be upgraded before attempting the cluster upgrade
  let remediation = match version_skew {
    1 | 2 => Remediation::Recommended,
    _ => Remediation::Required,
  };

  Ok(vec![KubeProxyVersionSkew {
    finding: Finding::new(Code::K8S011, remediation),
    api_server_version: format!("v1.{control_plane_minor_version}"),
    kube_proxy_version: format!("v1.{kproxy_minor_version}"),
    version_skew: format!("{version_skew}"),
  }])
}

finding::impl_findings!(KubeProxyVersionSkew, "✅ - `kube-proxy` version is aligned with the node/`kubelet` versions in use");

pub trait K8sFindings {
  fn get_resource(&self) -> Resource;

  /// K8S002 - check if resources contain a minimum of 3 replicas
  fn min_replicas(&self) -> Option<MinReplicas>;

  /// K8S003 - check if resources contain minReadySeconds > 0
  fn min_ready_seconds(&self) -> Option<MinReadySeconds>;

  // /// K8S004 - check if resources have associated podDisruptionBudgets
  // fn pod_disruption_budget(&self) -> Option<PodDisruptionBudget>;

  /// K8S005 - check if resources have podAntiAffinity or topologySpreadConstraints
  fn pod_topology_distribution(&self) -> Option<PodTopologyDistribution>;

  /// K8S006 - check if resources have readinessProbe
  fn readiness_probe(&self) -> Option<Probe>;

  /// K8S007 - check if StatefulSets have terminationGracePeriodSeconds == 0
  fn termination_grace_period(&self) -> Option<TerminationGracePeriod>;

  /// K8S008 - check if resources use the Docker socket
  fn docker_socket(&self, target_version: &str) -> Result<Option<DockerSocket>>;

  // K8S009 - pod security policies (separate from workload resources)
}
