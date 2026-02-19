use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tabled::{
  Table, Tabled,
  settings::{Margin, Remove, Style, location::ByColumnName},
};

use k8s_openapi::api::core::v1::ConfigMap;

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

/// Returns version skew findings for all nodes in the cluster
pub fn version_skew(nodes: &[resources::Node], control_plane_minor: i32) -> Vec<VersionSkew> {
  let mut findings = vec![];

  for node in nodes {
    let skew = control_plane_minor - node.minor_version;
    if skew <= 0 {
      continue;
    }

    // Prior to upgrade, the node version (kubelet) should not be more than 3 version behind
    // the control plane version (api server). If it is, the node must be upgraded before
    // attempting the cluster upgrade
    let remediation = match skew {
      1 | 2 => Remediation::Recommended,
      _ => Remediation::Required,
    };

    findings.push(VersionSkew {
      finding: Finding::new(Code::K8S001, remediation),
      name: node.name.to_owned(),
      kubelet_version: node.kubelet_version.to_owned(),
      kubernetes_version: format!("v{}", version::format_version(node.minor_version)),
      control_plane_version: format!("v{}", version::format_version(control_plane_minor)),
      version_skew: format!("+{skew}"),
    });
  }

  findings
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
  control_plane_minor: i32,
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

  let version_skew = control_plane_minor - kproxy_minor_version;
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
    api_server_version: format!("v1.{control_plane_minor}"),
    kube_proxy_version: format!("v1.{kproxy_minor_version}"),
    version_skew: format!("{version_skew}"),
  }])
}

finding::impl_findings!(KubeProxyVersionSkew, "✅ - `kube-proxy` version is aligned with the node/`kubelet` versions in use");

#[derive(Clone, Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct KubeProxyIpvsMode {
  #[tabled(inline)]
  pub finding: finding::Finding,
  #[tabled(rename = "CURRENT MODE")]
  pub current_mode: String,
}

finding::impl_findings!(KubeProxyIpvsMode, "✅ - `kube-proxy` is not using the deprecated IPVS mode");

/// Check if kube-proxy is configured with IPVS mode, which is deprecated in 1.35 and
/// removed in 1.36
pub fn kube_proxy_ipvs_mode(
  configmap: Option<&ConfigMap>,
  target_minor: i32,
) -> Result<Vec<KubeProxyIpvsMode>> {
  if target_minor < 35 {
    return Ok(vec![]);
  }

  let cm = match configmap {
    Some(cm) => cm,
    None => return Ok(vec![]),
  };

  let data = match &cm.data {
    Some(data) => data,
    None => return Ok(vec![]),
  };

  let config_str = match data.get("config") {
    Some(c) => c,
    None => return Ok(vec![]),
  };

  let config: serde_yaml::Value = serde_yaml::from_str(config_str)
    .context("Failed to parse kube-proxy-config ConfigMap as YAML")?;

  let mode = config.get("mode")
    .and_then(|v| v.as_str())
    .unwrap_or("");

  if mode == "ipvs" {
    let remediation = if target_minor >= 36 {
      Remediation::Required
    } else {
      Remediation::Recommended
    };

    Ok(vec![KubeProxyIpvsMode {
      finding: Finding::new(Code::K8S012, remediation),
      current_mode: mode.to_owned(),
    }])
  } else {
    Ok(vec![])
  }
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct IngressNginxRetirement {
  #[tabled(inline)]
  pub finding: finding::Finding,
  #[tabled(inline)]
  pub resource: Resource,
  pub image: String,
}

finding::impl_findings!(IngressNginxRetirement, "✅ - No Ingress NGINX controller images detected that require migration");

/// Check for the retired Kubernetes community Ingress NGINX controller images
/// which are no longer maintained as of 1.35+
pub fn ingress_nginx_retirement(
  resources: &[resources::StdResource],
  target_minor: i32,
) -> Result<Vec<IngressNginxRetirement>> {
  if target_minor < 35 {
    return Ok(vec![]);
  }

  let mut findings = Vec::new();

  for resource in resources {
    // Only check Deployments and DaemonSets
    if !matches!(resource.metadata.kind, resources::Kind::Deployment | resources::Kind::DaemonSet) {
      continue;
    }

    let ptmpl = match resource.spec.template.as_ref() {
      Some(t) => t,
      None => continue,
    };
    let pspec = match ptmpl.spec.as_ref() {
      Some(s) => s,
      None => continue,
    };

    for container in &pspec.containers {
      let image = match &container.image {
        Some(img) => img,
        None => continue,
      };

      if image.contains("registry.k8s.io/ingress-nginx/controller")
        || image.contains("k8s.gcr.io/ingress-nginx/controller")
      {
        findings.push(IngressNginxRetirement {
          finding: Finding::new(Code::K8S013, Remediation::Recommended),
          resource: Resource {
            name: resource.metadata.name.to_owned(),
            namespace: resource.metadata.namespace.to_owned(),
            kind: resource.metadata.kind.to_owned(),
          },
          image: image.to_owned(),
        });
      }
    }
  }

  Ok(findings)
}

pub trait K8sFindings {
  fn get_resource(&self) -> Resource;

  /// K8S002 - check if resources contain a minimum of 3 replicas
  fn min_replicas(&self) -> Option<MinReplicas>;

  /// K8S003 - check if resources contain minReadySeconds > 0
  fn min_ready_seconds(&self) -> Option<MinReadySeconds>;

  /// K8S005 - check if resources have podAntiAffinity or topologySpreadConstraints
  fn pod_topology_distribution(&self) -> Option<PodTopologyDistribution>;

  /// K8S006 - check if resources have readinessProbe
  fn readiness_probe(&self) -> Option<Probe>;

  /// K8S007 - check if StatefulSets have terminationGracePeriodSeconds == 0
  fn termination_grace_period(&self) -> Option<TerminationGracePeriod>;

  /// K8S008 - check if resources use the Docker socket
  fn docker_socket(&self) -> Result<Option<DockerSocket>>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::BTreeMap;
  use k8s_openapi::api::core::v1::{Container, PodSpec, PodTemplateSpec};

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  fn make_node(name: &str, minor_version: i32) -> resources::Node {
    resources::Node {
      name: name.to_string(),
      labels: None,
      kubelet_version: format!("v1.{minor_version}.0"),
      minor_version,
    }
  }

  fn make_kube_proxy_daemonset(image: &str) -> resources::StdResource {
    resources::StdResource {
      metadata: resources::StdMetadata {
        name: "kube-proxy".to_string(),
        namespace: "kube-system".to_string(),
        kind: resources::Kind::DaemonSet,
        labels: BTreeMap::new(),
        annotations: BTreeMap::new(),
      },
      spec: resources::StdSpec {
        min_ready_seconds: None,
        replicas: None,
        template: Some(PodTemplateSpec {
          metadata: None,
          spec: Some(PodSpec {
            containers: vec![Container {
              name: "kube-proxy".to_string(),
              image: Some(image.to_string()),
              ..Container::default()
            }],
            ..PodSpec::default()
          }),
        }),
      },
    }
  }

  fn make_configmap(yaml_str: &str) -> ConfigMap {
    ConfigMap {
      data: Some(BTreeMap::from([("config".to_string(), yaml_str.to_string())])),
      ..ConfigMap::default()
    }
  }

  fn make_deployment_with_image(name: &str, namespace: &str, image: &str) -> resources::StdResource {
    resources::StdResource {
      metadata: resources::StdMetadata {
        name: name.to_string(),
        namespace: namespace.to_string(),
        kind: resources::Kind::Deployment,
        labels: BTreeMap::new(),
        annotations: BTreeMap::new(),
      },
      spec: resources::StdSpec {
        min_ready_seconds: None,
        replicas: Some(1),
        template: Some(PodTemplateSpec {
          metadata: None,
          spec: Some(PodSpec {
            containers: vec![Container {
              name: "controller".to_string(),
              image: Some(image.to_string()),
              ..Container::default()
            }],
            ..PodSpec::default()
          }),
        }),
      },
    }
  }

  fn make_daemonset_with_image(name: &str, namespace: &str, image: &str) -> resources::StdResource {
    resources::StdResource {
      metadata: resources::StdMetadata {
        name: name.to_string(),
        namespace: namespace.to_string(),
        kind: resources::Kind::DaemonSet,
        labels: BTreeMap::new(),
        annotations: BTreeMap::new(),
      },
      spec: resources::StdSpec {
        min_ready_seconds: None,
        replicas: None,
        template: Some(PodTemplateSpec {
          metadata: None,
          spec: Some(PodSpec {
            containers: vec![Container {
              name: "controller".to_string(),
              image: Some(image.to_string()),
              ..Container::default()
            }],
            ..PodSpec::default()
          }),
        }),
      },
    }
  }

  // ===========================================================================
  // version_skew
  // ===========================================================================

  #[test]
  fn version_skew_empty_nodes() {
    let result = version_skew(&[], 30);
    assert!(result.is_empty());
  }

  #[test]
  fn version_skew_no_skew_same_version() {
    let nodes = vec![make_node("node-1", 30)];
    let result = version_skew(&nodes, 30);
    assert!(result.is_empty(), "node at same version should produce no findings");
  }

  #[test]
  fn version_skew_node_ahead_of_control_plane() {
    let nodes = vec![make_node("node-1", 31)];
    let result = version_skew(&nodes, 30);
    assert!(result.is_empty(), "node ahead of control plane should be skipped");
  }

  #[test]
  fn version_skew_1_recommended() {
    let nodes = vec![make_node("node-1", 29)];
    let result = version_skew(&nodes, 30);
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Recommended));
    assert_eq!(result[0].version_skew, "+1");
  }

  #[test]
  fn version_skew_2_recommended() {
    let nodes = vec![make_node("node-1", 28)];
    let result = version_skew(&nodes, 30);
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Recommended));
    assert_eq!(result[0].version_skew, "+2");
  }

  #[test]
  fn version_skew_3_plus_required() {
    let nodes = vec![make_node("node-1", 27)];
    let result = version_skew(&nodes, 30);
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Required));
    assert_eq!(result[0].version_skew, "+3");
  }

  #[test]
  fn version_skew_multiple_mixed_nodes() {
    let nodes = vec![
      make_node("same", 30),      // skew 0 -> skipped
      make_node("ahead", 31),     // ahead -> skipped
      make_node("behind-1", 29),  // skew 1 -> Recommended
      make_node("behind-3", 27),  // skew 3 -> Required
      make_node("behind-4", 26),  // skew 4 -> Required
    ];
    let result = version_skew(&nodes, 30);
    assert_eq!(result.len(), 3);

    // behind-1
    assert_eq!(result[0].name, "behind-1");
    assert!(matches!(result[0].finding.remediation, Remediation::Recommended));
    assert_eq!(result[0].version_skew, "+1");

    // behind-3
    assert_eq!(result[1].name, "behind-3");
    assert!(matches!(result[1].finding.remediation, Remediation::Required));
    assert_eq!(result[1].version_skew, "+3");

    // behind-4
    assert_eq!(result[2].name, "behind-4");
    assert!(matches!(result[2].finding.remediation, Remediation::Required));
    assert_eq!(result[2].version_skew, "+4");
  }

  // ===========================================================================
  // kube_proxy_version_skew
  // ===========================================================================

  #[test]
  fn kube_proxy_version_skew_no_daemonset() {
    let resources: Vec<resources::StdResource> = vec![];
    let result = kube_proxy_version_skew(&resources, 30).unwrap();
    assert!(result.is_empty(), "no kube-proxy daemonset should return empty");
  }

  #[test]
  fn kube_proxy_version_skew_no_skew() {
    let ds = make_kube_proxy_daemonset(
      "602401143452.dkr.ecr.us-east-1.amazonaws.com/eks/kube-proxy:v1.30.0-eksbuild.3",
    );
    let result = kube_proxy_version_skew(&[ds], 30).unwrap();
    assert!(result.is_empty(), "same version should produce no findings");
  }

  #[test]
  fn kube_proxy_version_skew_1_recommended() {
    let ds = make_kube_proxy_daemonset(
      "602401143452.dkr.ecr.us-east-1.amazonaws.com/eks/kube-proxy:v1.29.0-eksbuild.3",
    );
    let result = kube_proxy_version_skew(&[ds], 30).unwrap();
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Recommended));
    assert_eq!(result[0].version_skew, "1");
  }

  #[test]
  fn kube_proxy_version_skew_3_required() {
    let ds = make_kube_proxy_daemonset(
      "602401143452.dkr.ecr.us-east-1.amazonaws.com/eks/kube-proxy:v1.27.0-eksbuild.3",
    );
    let result = kube_proxy_version_skew(&[ds], 30).unwrap();
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Required));
    assert_eq!(result[0].version_skew, "3");
  }

  #[test]
  fn kube_proxy_version_skew_node_ahead() {
    let ds = make_kube_proxy_daemonset(
      "602401143452.dkr.ecr.us-east-1.amazonaws.com/eks/kube-proxy:v1.31.0-eksbuild.3",
    );
    let result = kube_proxy_version_skew(&[ds], 30).unwrap();
    assert!(result.is_empty(), "kube-proxy ahead of control plane should return empty");
  }

  // ===========================================================================
  // kube_proxy_ipvs_mode
  // ===========================================================================

  #[test]
  fn kube_proxy_ipvs_mode_none_configmap() {
    let result = kube_proxy_ipvs_mode(None, 35).unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn kube_proxy_ipvs_mode_no_config_key() {
    let cm = ConfigMap {
      data: Some(BTreeMap::from([("other-key".to_string(), "value".to_string())])),
      ..ConfigMap::default()
    };
    let result = kube_proxy_ipvs_mode(Some(&cm), 35).unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn kube_proxy_ipvs_mode_iptables() {
    let cm = make_configmap("mode: iptables");
    let result = kube_proxy_ipvs_mode(Some(&cm), 35).unwrap();
    assert!(result.is_empty(), "iptables mode should produce no findings");
  }

  #[test]
  fn kube_proxy_ipvs_mode_ipvs_target_35_recommended() {
    let cm = make_configmap("mode: ipvs");
    let result = kube_proxy_ipvs_mode(Some(&cm), 35).unwrap();
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Recommended));
    assert_eq!(result[0].current_mode, "ipvs");
  }

  #[test]
  fn kube_proxy_ipvs_mode_ipvs_target_36_required() {
    let cm = make_configmap("mode: ipvs");
    let result = kube_proxy_ipvs_mode(Some(&cm), 36).unwrap();
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Required));
    assert_eq!(result[0].current_mode, "ipvs");
  }

  // ===========================================================================
  // ingress_nginx_retirement
  // ===========================================================================

  #[test]
  fn ingress_nginx_retirement_empty_resources() {
    let result = ingress_nginx_retirement(&[], 35).unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn ingress_nginx_retirement_no_nginx_images() {
    let deploy = make_deployment_with_image("my-app", "default", "nginx:1.25");
    let result = ingress_nginx_retirement(&[deploy], 35).unwrap();
    assert!(result.is_empty(), "plain nginx image should not trigger findings");
  }

  #[test]
  fn ingress_nginx_retirement_registry_k8s_io_image() {
    let deploy = make_deployment_with_image(
      "ingress-nginx-controller",
      "ingress-nginx",
      "registry.k8s.io/ingress-nginx/controller:v1.9.0",
    );
    let result = ingress_nginx_retirement(&[deploy], 35).unwrap();
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Recommended));
    assert_eq!(result[0].image, "registry.k8s.io/ingress-nginx/controller:v1.9.0");
    assert_eq!(result[0].resource.name, "ingress-nginx-controller");
  }

  #[test]
  fn ingress_nginx_retirement_k8s_gcr_io_image() {
    let deploy = make_deployment_with_image(
      "ingress-nginx-controller",
      "ingress-nginx",
      "k8s.gcr.io/ingress-nginx/controller:v1.5.1",
    );
    let result = ingress_nginx_retirement(&[deploy], 35).unwrap();
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].finding.remediation, Remediation::Recommended));
    assert_eq!(result[0].image, "k8s.gcr.io/ingress-nginx/controller:v1.5.1");
  }

  #[test]
  fn ingress_nginx_retirement_target_below_35() {
    let deploy = make_deployment_with_image(
      "ingress-nginx-controller",
      "ingress-nginx",
      "registry.k8s.io/ingress-nginx/controller:v1.9.0",
    );
    let result = ingress_nginx_retirement(&[deploy], 34).unwrap();
    assert!(result.is_empty(), "target below 35 should produce no findings");
  }

  #[test]
  fn ingress_nginx_retirement_multiple_findings() {
    let deploy1 = make_deployment_with_image(
      "ingress-nginx-controller",
      "ingress-nginx",
      "registry.k8s.io/ingress-nginx/controller:v1.9.0",
    );
    let deploy2 = make_daemonset_with_image(
      "ingress-nginx-controller-legacy",
      "legacy-ns",
      "k8s.gcr.io/ingress-nginx/controller:v1.5.1",
    );
    // A non-matching resource that should be ignored
    let deploy3 = make_deployment_with_image(
      "my-app",
      "default",
      "my-registry.io/my-app:v2.0",
    );
    let result = ingress_nginx_retirement(&[deploy1, deploy2, deploy3], 35).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].resource.name, "ingress-nginx-controller");
    assert_eq!(result[1].resource.name, "ingress-nginx-controller-legacy");
  }
}
