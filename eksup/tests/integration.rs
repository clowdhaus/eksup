mod common;

use common::{fixtures, mock_k8s::MockK8sClients};
use eksup::eks::resources::VpcSubnet;

// ============================================================================
// Cluster findings
// ============================================================================

#[tokio::test]
async fn cluster_findings_healthy() {
  let aws = fixtures::healthy_aws();
  let result = eksup::eks::get_cluster_findings(&aws.cluster).unwrap();
  assert!(result.cluster_health.is_empty());
}

#[tokio::test]
async fn cluster_findings_with_health_issues() {
  use aws_sdk_eks::types::{Cluster, ClusterHealth, ClusterIssue, ClusterIssueCode};

  let cluster = Cluster::builder()
    .name("test")
    .version("1.30")
    .health(
      ClusterHealth::builder()
        .issues(
          ClusterIssue::builder()
            .code(ClusterIssueCode::Ec2SubnetNotFound)
            .message("Subnet not found")
            .resource_ids("subnet-123")
            .build(),
        )
        .build(),
    )
    .build();

  let result = eksup::eks::get_cluster_findings(&cluster).unwrap();
  assert_eq!(result.cluster_health.len(), 1);
}

// ============================================================================
// Subnet findings
// ============================================================================

#[tokio::test]
async fn subnet_findings_sufficient_ips() {
  let aws = fixtures::healthy_aws();
  let k8s = fixtures::healthy_k8s();
  let result = eksup::eks::get_subnet_findings(&aws, &k8s, &aws.cluster).await.unwrap();
  assert!(result.control_plane_ips.is_empty());
  assert!(result.pod_ips.is_empty());
}

#[tokio::test]
async fn subnet_findings_insufficient_control_plane_ips() {
  let mut aws = fixtures::healthy_aws();
  aws.subnet_ips = vec![
    VpcSubnet { id: "subnet-1".into(), available_ips: 3, availability_zone_id: "use1-az1".into() },
    VpcSubnet { id: "subnet-2".into(), available_ips: 2, availability_zone_id: "use1-az2".into() },
  ];
  let k8s = fixtures::healthy_k8s();

  let result = eksup::eks::get_subnet_findings(&aws, &k8s, &aws.cluster).await.unwrap();
  assert!(!result.control_plane_ips.is_empty(), "should report insufficient IPs");
}

// ============================================================================
// Addon findings
// ============================================================================

#[tokio::test]
async fn addon_findings_no_addons() {
  let aws = fixtures::healthy_aws();
  let result = eksup::eks::get_addon_findings(&aws, "test-cluster", "1.30", 31).await.unwrap();
  assert!(result.version_compatibility.is_empty());
  assert!(result.health.is_empty());
}

#[tokio::test]
async fn addon_findings_version_incompatible() {
  use aws_sdk_eks::types::Addon;
  use std::collections::HashMap;

  let mut aws = fixtures::healthy_aws();
  aws.addons = vec![
    Addon::builder().addon_name("vpc-cni").addon_version("v1.12.0").build(),
  ];
  aws.addon_versions = HashMap::from([
    (("vpc-cni".into(), "1.30".into()), fixtures::make_addon_version("v1.15.0", "v1.14.0", &["v1.15.0", "v1.14.0"])),
    (("vpc-cni".into(), "1.31".into()), fixtures::make_addon_version("v1.16.0", "v1.15.0", &["v1.16.0", "v1.15.0"])),
  ]);

  let result = eksup::eks::get_addon_findings(&aws, "test-cluster", "1.30", 31).await.unwrap();
  assert_eq!(result.version_compatibility.len(), 1);
}

// ============================================================================
// Data plane findings
// ============================================================================

#[tokio::test]
async fn data_plane_findings_empty() {
  let aws = fixtures::healthy_aws();
  let result = eksup::eks::get_data_plane_findings(&aws, &aws.cluster, 31).await.unwrap();
  assert!(result.eks_managed_nodegroup_health.is_empty());
  assert!(result.eks_managed_nodegroup_update.is_empty());
  assert!(result.self_managed_nodegroup_update.is_empty());
}

#[tokio::test]
async fn data_plane_findings_node_ips() {
  use aws_sdk_eks::types::Nodegroup;

  let mut aws = fixtures::healthy_aws();
  aws.nodegroups = vec![
    Nodegroup::builder()
      .nodegroup_name("test-ng")
      .subnets("subnet-1")
      .subnets("subnet-2")
      .build(),
  ];
  aws.subnet_ips = vec![
    VpcSubnet { id: "subnet-1".into(), available_ips: 10, availability_zone_id: "use1-az1".into() },
    VpcSubnet { id: "subnet-2".into(), available_ips: 10, availability_zone_id: "use1-az2".into() },
  ];

  let result = eksup::eks::get_data_plane_findings(&aws, &aws.cluster, 31).await.unwrap();
  assert!(!result.node_ips.is_empty(), "low IP subnets should produce findings");
}

// ============================================================================
// Kubernetes findings
// ============================================================================

#[tokio::test]
async fn kubernetes_findings_empty() {
  let k8s = fixtures::healthy_k8s();
  let result = eksup::k8s::get_kubernetes_findings(&k8s, 30, 31).await.unwrap();
  assert!(result.version_skew.is_empty());
  assert!(result.min_replicas.is_empty());
}

#[tokio::test]
async fn kubernetes_findings_version_skew() {
  let k8s = MockK8sClients {
    nodes: vec![fixtures::make_node("node-1", 28)],
    ..Default::default()
  };

  let result = eksup::k8s::get_kubernetes_findings(&k8s, 30, 31).await.unwrap();
  assert_eq!(result.version_skew.len(), 1);
}

#[tokio::test]
async fn kubernetes_findings_workload_issues() {
  let k8s = MockK8sClients {
    resources: vec![fixtures::make_deployment("web", "default", 1)],
    ..Default::default()
  };

  let result = eksup::k8s::get_kubernetes_findings(&k8s, 30, 31).await.unwrap();
  assert!(!result.min_replicas.is_empty(), "1 replica should trigger finding");
  assert!(!result.readiness_probe.is_empty(), "missing probe should trigger finding");
}

#[tokio::test]
async fn kubernetes_findings_missing_pdb() {
  use std::collections::BTreeMap;
  use k8s_openapi::api::core::v1::{Container, PodSpec, PodTemplateSpec};
  use eksup::k8s::resources::{Kind, StdMetadata, StdResource, StdSpec};

  let labels = BTreeMap::from([("app".to_string(), "web".to_string())]);
  let deploy = StdResource {
    metadata: StdMetadata {
      name: "web".into(),
      namespace: "default".into(),
      kind: Kind::Deployment,
      labels: BTreeMap::new(),
      annotations: BTreeMap::new(),
    },
    spec: StdSpec {
      min_ready_seconds: None,
      replicas: Some(3),
      template: Some(PodTemplateSpec {
        metadata: Some(k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
          labels: Some(labels),
          ..Default::default()
        }),
        spec: Some(PodSpec {
          containers: vec![Container { name: "app".into(), ..Default::default() }],
          ..Default::default()
        }),
      }),
    },
  };

  let k8s = MockK8sClients {
    resources: vec![deploy],
    ..Default::default()
  };

  let result = eksup::k8s::get_kubernetes_findings(&k8s, 30, 31).await.unwrap();
  assert!(!result.pod_disruption_budgets.is_empty(), "missing PDB should trigger finding");
}

// ============================================================================
// Full analysis pipeline
// ============================================================================

#[tokio::test]
async fn analyze_healthy_cluster() {
  let aws = fixtures::healthy_aws();
  let k8s = fixtures::healthy_k8s();
  let results = eksup::analysis::analyze(&aws, &k8s, &aws.cluster, 31).await.unwrap();

  assert!(results.cluster.cluster_health.is_empty());
  assert!(results.subnets.control_plane_ips.is_empty());
  assert!(results.addons.version_compatibility.is_empty());
  assert!(results.kubernetes.version_skew.is_empty());
}

#[tokio::test]
async fn analyze_filter_recommended() {
  let k8s = MockK8sClients {
    resources: vec![fixtures::make_deployment("web", "default", 2)],
    nodes: vec![fixtures::make_node("node-1", 29)],
    ..Default::default()
  };
  let aws = fixtures::healthy_aws();

  let mut results = eksup::analysis::analyze(&aws, &k8s, &aws.cluster, 31).await.unwrap();
  let before_skew = results.kubernetes.version_skew.len();
  results.filter_recommended();
  // Version skew of 1 is Recommended, so it should be filtered out
  assert!(results.kubernetes.version_skew.len() < before_skew || before_skew == 0);
}

// ============================================================================
// Explicit target version
// ============================================================================

#[tokio::test]
async fn analyze_with_explicit_target() {
  let aws = fixtures::healthy_aws(); // cluster version "1.30"
  let k8s = fixtures::healthy_k8s();

  // Jump from 1.30 → 1.33
  let results = eksup::analysis::analyze(&aws, &k8s, &aws.cluster, 33).await.unwrap();

  assert!(results.cluster.cluster_health.is_empty());
  assert!(results.subnets.control_plane_ips.is_empty());
}

// ============================================================================
// Error paths
// ============================================================================

#[tokio::test]
async fn analyze_aws_error_propagates() {
  use common::mock_aws::MockAwsClientsError;
  use common::mock_k8s::MockK8sClientsError;

  let cluster = aws_sdk_eks::types::Cluster::builder()
    .name("test")
    .version("1.30")
    .build();

  let result = eksup::analysis::analyze(&MockAwsClientsError, &MockK8sClientsError, &cluster, 31).await;
  assert!(result.is_err(), "should propagate AWS/K8s errors");
}
