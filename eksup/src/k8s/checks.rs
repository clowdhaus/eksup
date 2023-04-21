use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tabled::{
  settings::{locator::ByColumnName, Disable, Margin, Style},
  Table, Tabled,
};

use crate::{
  finding::{self, Findings},
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
      .with(Disable::column(ByColumnName::new("String")))
      .with(Disable::column(ByColumnName::new("NAME")))
      .with(Style::markdown());

    let mut table = Table::new(self);
    table
      .with(Disable::column(ByColumnName::new("CHECK")))
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
pub async fn version_skew(nodes: &[resources::Node], cluster_version: &str) -> Result<Vec<VersionSkew>> {
  let mut findings = vec![];

  for node in nodes {
    let control_plane_minor_version = version::parse_minor(cluster_version)?;
    let version_skew = control_plane_minor_version - node.minor_version;
    if version_skew == 0 {
      continue;
    }

    // Prior to upgrade, the node version should not be more than 1 version behind
    // the control plane version. If it is, the node must be upgraded before
    // attempting the cluster upgrade
    let mut remediation = match version_skew {
      1 => finding::Remediation::Recommended,
      _ => finding::Remediation::Required,
    };

    if let Some(labels) = &node.labels {
      if labels.contains_key("eks.amazonaws.com/nodegroup") {
        // Nodes created by EKS managed nodegroups are required to match control plane
        // before the control plane will permit an upgrade
        remediation = finding::Remediation::Required;
      }
    }

    if node.name.starts_with("fargate-") {
      // Nodes created by EKS Fargate are required to match control plane
      // before the control plane will permit an upgrade
      remediation = finding::Remediation::Required;
    }

    let finding = finding::Finding {
      code: finding::Code::K8S001,
      symbol: remediation.symbol(),
      remediation,
    };

    let node = VersionSkew {
      finding,
      name: node.name.to_owned(),
      kubelet_version: node.kubelet_version.to_owned(),
      kubernetes_version: format!("v{}", version::normalize(&node.kubelet_version).unwrap()),
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

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct PodDisruptionBudget {
  #[tabled(inline)]
  pub finding: finding::Finding,
  #[tabled(inline)]
  pub resource: Resource,
  // Has pod associated pod disruption budget
  // TODO - more relevant information than just present?
}

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

impl Findings for Vec<PodTopologyDistribution> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - All relevant Kubernetes workloads have either podAntiAffinity or topologySpreadConstraints set"
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

impl Findings for Vec<TerminationGracePeriod> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - No StatefulSet workloads have a terminationGracePeriodSeconds set to more than 0"
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

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct DockerSocket {
  #[tabled(inline)]
  pub finding: finding::Finding,

  #[tabled(inline)]
  pub resource: Resource,

  pub docker_socket: bool,
}

impl Findings for Vec<DockerSocket> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - No relevant Kubernetes workloads are found to be utilizing the Docker socket"
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

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct PodSecurityPolicy {
  #[tabled(inline)]
  pub finding: finding::Finding,

  #[tabled(inline)]
  pub resource: Resource,
}

impl Findings for Vec<PodSecurityPolicy> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - No PodSecurityPolicys were found within the cluster"
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

#[derive(Clone, Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct KubeProxyVersionSkew {
  #[tabled(inline)]
  pub finding: finding::Finding,
  #[tabled(rename = "KUBELET")]
  pub kubelet_version: String,
  #[tabled(rename = "KUBE PROXY")]
  pub kube_proxy_version: String,
  #[tabled(rename = "SKEW")]
  pub version_skew: String,
}

pub async fn kube_proxy_version_skew(
  nodes: &[resources::Node],
  resources: &[resources::StdResource],
) -> Result<Vec<KubeProxyVersionSkew>> {
  let kube_proxy = match resources
    .iter()
    .filter(|r| r.metadata.kind == resources::Kind::DaemonSet && r.metadata.name == "kube-proxy")
    .collect::<Vec<_>>()
    .get(0)
  {
    Some(k) => k.to_owned(),
    None => {
      println!("Unable to find kube-proxy");
      return Ok(vec![]);
    }
  };

  let ptmpl = kube_proxy.spec.template.as_ref().unwrap();
  let pspec = ptmpl.spec.as_ref().unwrap();
  let kproxy_minor_version = pspec
    .containers
    .iter()
    .map(|container| {
      // TODO - this seems brittle
      let image = container.image.as_ref().unwrap().split(':').collect::<Vec<_>>()[1];
      version::parse_minor(image).unwrap()
    })
    .next()
    .context("Unable to find image version for kube-proxy")?;

  let findings = nodes
    .iter()
    .map(|node| node.minor_version)
    .collect::<HashSet<_>>()
    .into_iter()
    .filter(|node_ver| node_ver != &kproxy_minor_version)
    .map(|node_ver| {
      let remediation = finding::Remediation::Required;
      let finding = finding::Finding {
        code: finding::Code::K8S011,
        symbol: remediation.symbol(),
        remediation,
      };

      KubeProxyVersionSkew {
        finding,
        kubelet_version: format!("v1.{node_ver}"),
        kube_proxy_version: format!("v1.{kproxy_minor_version}"),
        version_skew: format!("{}", kproxy_minor_version - node_ver),
      }
    })
    .collect();

  Ok(findings)
}

impl Findings for Vec<KubeProxyVersionSkew> {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String> {
    if self.is_empty() {
      return Ok(format!(
        "{leading_whitespace}✅ - `kube-proxy` version is aligned with the node/`kubelet` versions in use"
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
  fn docker_socket(&self, target_version: &str) -> Option<DockerSocket>;

  // K8S009 - pod security policies (separate from workload resources)
}
