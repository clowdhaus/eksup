use std::collections::BTreeMap;

use aws_sdk_eks::types::{Cluster, ClusterHealth, VpcConfigResponse};
use k8s_openapi::api::core::v1::{Container, PodSpec, PodTemplateSpec};

use eksup::eks::resources::{AddonVersion, ClusterInsight, VpcSubnet};
use eksup::k8s::resources::{Kind, Node, StdMetadata, StdPdb, StdResource, StdSpec};

use super::mock_aws::MockAwsClients;
use super::mock_k8s::MockK8sClients;

/// Builds a healthy cluster at version 1.30 with sufficient IPs
pub fn healthy_aws() -> MockAwsClients {
  MockAwsClients {
    cluster: Cluster::builder()
      .name("test-cluster")
      .version("1.30")
      .health(ClusterHealth::builder().build())
      .resources_vpc_config(
        VpcConfigResponse::builder()
          .subnet_ids("subnet-1")
          .subnet_ids("subnet-2")
          .build(),
      )
      .build(),
    subnet_ips: vec![
      VpcSubnet { id: "subnet-1".into(), available_ips: 100, availability_zone_id: "use1-az1".into() },
      VpcSubnet { id: "subnet-2".into(), available_ips: 100, availability_zone_id: "use1-az2".into() },
    ],
    ..Default::default()
  }
}

/// Builds a minimal K8s mock with no resources
pub fn healthy_k8s() -> MockK8sClients {
  MockK8sClients::default()
}

/// Creates a Node with the given version
pub fn make_node(name: &str, minor: i32) -> Node {
  Node {
    name: name.into(),
    labels: None,
    kubelet_version: format!("v1.{minor}.0"),
    minor_version: minor,
  }
}

/// Creates a simple Deployment StdResource
pub fn make_deployment(name: &str, namespace: &str, replicas: i32) -> StdResource {
  StdResource {
    metadata: StdMetadata {
      name: name.into(),
      namespace: namespace.into(),
      kind: Kind::Deployment,
      labels: BTreeMap::new(),
      annotations: BTreeMap::new(),
    },
    spec: StdSpec {
      min_ready_seconds: None,
      replicas: Some(replicas),
      template: Some(PodTemplateSpec {
        metadata: None,
        spec: Some(PodSpec {
          containers: vec![Container {
            name: "app".into(),
            ..Default::default()
          }],
          ..Default::default()
        }),
      }),
    },
  }
}

/// Creates a PDB that matches pods with the given labels
pub fn make_pdb(
  name: &str,
  namespace: &str,
  match_labels: BTreeMap<String, String>,
  has_min_available: bool,
  has_max_unavailable: bool,
) -> StdPdb {
  use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
  use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
  StdPdb {
    name: name.into(),
    namespace: namespace.into(),
    selector: Some(LabelSelector {
      match_labels: Some(match_labels),
      ..Default::default()
    }),
    min_available: if has_min_available { Some(IntOrString::Int(1)) } else { None },
    max_unavailable: if has_max_unavailable { Some(IntOrString::Int(1)) } else { None },
  }
}

/// Creates an AddonVersion for mock responses
pub fn make_addon_version(latest: &str, default: &str, supported: &[&str]) -> AddonVersion {
  AddonVersion {
    latest: latest.into(),
    default: default.into(),
    supported_versions: supported.iter().map(|s| s.to_string()).collect(),
  }
}

/// Creates a ClusterInsight for mock responses
pub fn make_insight(
  id: &str,
  name: &str,
  category: &str,
  status: &str,
  kubernetes_version: &str,
  description: &str,
  recommendation: &str,
) -> ClusterInsight {
  ClusterInsight {
    id: id.into(),
    name: name.into(),
    category: category.into(),
    status: status.into(),
    status_reason: String::new(),
    kubernetes_version: kubernetes_version.into(),
    description: description.into(),
    recommendation: recommendation.into(),
  }
}
