mod common;

use aws_sdk_eks::types::{Cluster, ClusterHealth, FargateProfile, Nodegroup, VpcConfigResponse};
use common::{fixtures, mock_aws::MockAwsClients, mock_k8s::MockK8sClients};
use eksup::analysis::Results;
use eksup::eks::resources::VpcSubnet;

/// Helper: run analysis and return Results
async fn run_analysis(aws: &MockAwsClients, k8s: &MockK8sClients) -> Results {
  eksup::analysis::analyze(aws, k8s, &aws.cluster).await.unwrap()
}

/// Helper: render Results as text
fn render_text(results: &Results) -> String {
  results.to_stdout_table().unwrap()
}

/// Helper: render Results as JSON
fn render_json(results: &Results) -> String {
  serde_json::to_string_pretty(results).unwrap()
}

// ============================================================================
// Healthy cluster
// ============================================================================

#[tokio::test]
async fn snapshot_healthy_cluster_text() {
  let aws = fixtures::healthy_aws();
  let k8s = fixtures::healthy_k8s();
  let results = run_analysis(&aws, &k8s).await;
  let output = render_text(&results);
  insta::assert_snapshot!("healthy_cluster_text", output);
}

#[tokio::test]
async fn snapshot_healthy_cluster_json() {
  let aws = fixtures::healthy_aws();
  let k8s = fixtures::healthy_k8s();
  let results = run_analysis(&aws, &k8s).await;
  let output = render_json(&results);
  insta::assert_snapshot!("healthy_cluster_json", output);
}

// ============================================================================
// Insufficient control plane IPs
// ============================================================================

#[tokio::test]
async fn snapshot_insufficient_control_plane_ips_text() {
  let mut aws = fixtures::healthy_aws();
  aws.subnet_ips = vec![
    VpcSubnet { id: "subnet-1".into(), available_ips: 3, availability_zone_id: "use1-az1".into() },
    VpcSubnet { id: "subnet-2".into(), available_ips: 2, availability_zone_id: "use1-az2".into() },
  ];
  let k8s = fixtures::healthy_k8s();
  let results = run_analysis(&aws, &k8s).await;
  let output = render_text(&results);
  insta::assert_snapshot!("insufficient_control_plane_ips_text", output);
}

// ============================================================================
// Workload best practices
// ============================================================================

#[tokio::test]
async fn snapshot_workload_issues_text() {
  let aws = fixtures::healthy_aws();
  let k8s = MockK8sClients {
    resources: vec![
      fixtures::make_deployment("web", "default", 1),
      fixtures::make_deployment("api", "backend", 2),
    ],
    ..Default::default()
  };
  let results = run_analysis(&aws, &k8s).await;
  let output = render_text(&results);
  insta::assert_snapshot!("workload_issues_text", output);
}

#[tokio::test]
async fn snapshot_workload_issues_json() {
  let aws = fixtures::healthy_aws();
  let k8s = MockK8sClients {
    resources: vec![
      fixtures::make_deployment("web", "default", 1),
      fixtures::make_deployment("api", "backend", 2),
    ],
    ..Default::default()
  };
  let results = run_analysis(&aws, &k8s).await;
  let output = render_json(&results);
  insta::assert_snapshot!("workload_issues_json", output);
}

// ============================================================================
// Version skew
// ============================================================================

#[tokio::test]
async fn snapshot_version_skew_text() {
  let aws = fixtures::healthy_aws();
  let k8s = MockK8sClients {
    nodes: vec![
      fixtures::make_node("node-1", 28),
      fixtures::make_node("node-2", 27),
    ],
    ..Default::default()
  };
  let results = run_analysis(&aws, &k8s).await;
  let output = render_text(&results);
  insta::assert_snapshot!("version_skew_text", output);
}

// ============================================================================
// Mixed findings
// ============================================================================

#[tokio::test]
async fn snapshot_mixed_findings_text() {
  let mut aws = fixtures::healthy_aws();
  aws.subnet_ips = vec![
    VpcSubnet { id: "subnet-1".into(), available_ips: 3, availability_zone_id: "use1-az1".into() },
    VpcSubnet { id: "subnet-2".into(), available_ips: 100, availability_zone_id: "use1-az2".into() },
  ];
  let k8s = MockK8sClients {
    nodes: vec![fixtures::make_node("node-1", 28)],
    resources: vec![fixtures::make_deployment("web", "default", 1)],
    ..Default::default()
  };

  let results = run_analysis(&aws, &k8s).await;
  let output = render_text(&results);
  insta::assert_snapshot!("mixed_findings_text", output);
}

#[tokio::test]
async fn snapshot_mixed_findings_json() {
  let mut aws = fixtures::healthy_aws();
  aws.subnet_ips = vec![
    VpcSubnet { id: "subnet-1".into(), available_ips: 3, availability_zone_id: "use1-az1".into() },
    VpcSubnet { id: "subnet-2".into(), available_ips: 100, availability_zone_id: "use1-az2".into() },
  ];
  let k8s = MockK8sClients {
    nodes: vec![fixtures::make_node("node-1", 28)],
    resources: vec![fixtures::make_deployment("web", "default", 1)],
    ..Default::default()
  };

  let results = run_analysis(&aws, &k8s).await;
  let output = render_json(&results);
  insta::assert_snapshot!("mixed_findings_json", output);
}

// ============================================================================
// Filter recommended
// ============================================================================

#[tokio::test]
async fn snapshot_filter_recommended_text() {
  let aws = fixtures::healthy_aws();
  let k8s = MockK8sClients {
    nodes: vec![fixtures::make_node("node-1", 29)],
    resources: vec![fixtures::make_deployment("web", "default", 2)],
    ..Default::default()
  };

  let mut results = run_analysis(&aws, &k8s).await;
  results.filter_recommended();
  let output = render_text(&results);
  insta::assert_snapshot!("filter_recommended_text", output);
}

// ============================================================================
// Playbook rendering
// ============================================================================

/// Helper: run analysis then render playbook markdown
async fn render_playbook(aws: &MockAwsClients, k8s: &MockK8sClients) -> String {
  let results = run_analysis(aws, k8s).await;
  eksup::playbook::render("us-east-1", &aws.cluster, results).unwrap()
}

/// Helper: build a MockAwsClients at a specific cluster version
fn aws_at_version(version: &str) -> MockAwsClients {
  MockAwsClients {
    cluster: Cluster::builder()
      .name("test-cluster")
      .version(version)
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

// ---------- Snapshot tests ----------

#[tokio::test]
async fn snapshot_playbook_healthy() {
  let aws = fixtures::healthy_aws();
  let k8s = fixtures::healthy_k8s();
  let output = render_playbook(&aws, &k8s).await;
  insta::assert_snapshot!("playbook_healthy", output);
}

#[tokio::test]
async fn snapshot_playbook_eks_managed_nodegroups() {
  let mut aws = fixtures::healthy_aws();
  aws.nodegroups = vec![
    Nodegroup::builder().nodegroup_name("mng-1").build(),
  ];
  let k8s = fixtures::healthy_k8s();
  let output = render_playbook(&aws, &k8s).await;
  insta::assert_snapshot!("playbook_eks_managed_nodegroups", output);
}

#[tokio::test]
async fn snapshot_playbook_fargate_profiles() {
  let mut aws = fixtures::healthy_aws();
  aws.fargate_profiles = vec![
    FargateProfile::builder().fargate_profile_name("fp-1").build(),
  ];
  let k8s = fixtures::healthy_k8s();
  let output = render_playbook(&aws, &k8s).await;
  insta::assert_snapshot!("playbook_fargate_profiles", output);
}

#[tokio::test]
async fn snapshot_playbook_mixed() {
  let mut aws = fixtures::healthy_aws();
  aws.nodegroups = vec![
    Nodegroup::builder().nodegroup_name("mng-1").build(),
  ];
  aws.fargate_profiles = vec![
    FargateProfile::builder().fargate_profile_name("fp-1").build(),
  ];
  let k8s = MockK8sClients {
    nodes: vec![
      fixtures::make_node("node-1", 28),
    ],
    resources: vec![
      fixtures::make_deployment("web", "default", 1),
    ],
    ..Default::default()
  };
  let output = render_playbook(&aws, &k8s).await;
  insta::assert_snapshot!("playbook_mixed", output);
}

// ---------- Conditional section assertion tests ----------

#[tokio::test]
async fn playbook_no_data_plane_sections_when_empty() {
  let aws = fixtures::healthy_aws();
  let k8s = fixtures::healthy_k8s();
  let output = render_playbook(&aws, &k8s).await;

  assert!(!output.contains("#### EKS Managed Nodegroup"), "EKS MNG sub-template section should be absent");
  assert!(!output.contains("#### Self-Managed Nodegroup"), "Self-managed sub-template section should be absent");
  assert!(!output.contains("### Fargate Node"), "Fargate sub-template section should be absent");
}

#[tokio::test]
async fn playbook_eks_managed_section_present_others_absent() {
  let mut aws = fixtures::healthy_aws();
  aws.nodegroups = vec![
    Nodegroup::builder().nodegroup_name("mng-1").build(),
  ];
  let k8s = fixtures::healthy_k8s();
  let output = render_playbook(&aws, &k8s).await;

  assert!(output.contains("#### EKS Managed Nodegroup"), "EKS MNG sub-template section should be present");
  assert!(!output.contains("#### Self-Managed Nodegroup"), "Self-managed sub-template section should be absent");
  assert!(!output.contains("### Fargate Node"), "Fargate sub-template section should be absent");
}

#[tokio::test]
async fn playbook_fargate_section_present_others_absent() {
  let mut aws = fixtures::healthy_aws();
  aws.fargate_profiles = vec![
    FargateProfile::builder().fargate_profile_name("fp-1").build(),
  ];
  let k8s = fixtures::healthy_k8s();
  let output = render_playbook(&aws, &k8s).await;

  assert!(output.contains("### Fargate Node"), "Fargate sub-template section should be present");
  assert!(!output.contains("#### EKS Managed Nodegroup"), "EKS MNG sub-template section should be absent");
  assert!(!output.contains("#### Self-Managed Nodegroup"), "Self-managed sub-template section should be absent");
}

#[tokio::test]
async fn playbook_template_variables_populated() {
  let aws = fixtures::healthy_aws();
  let k8s = fixtures::healthy_k8s();
  let output = render_playbook(&aws, &k8s).await;

  assert!(output.contains("test-cluster"), "cluster name should appear");
  assert!(output.contains("us-east-1"), "region should appear");
  assert!(output.contains("v1.30"), "current version should appear");
  assert!(output.contains("v1.31"), "target version should appear");
  assert!(output.contains("kubernetes.io/blog/"), "release URL should appear");
}

#[tokio::test]
async fn playbook_pod_ips_healthy_when_no_custom_networking() {
  let aws = fixtures::healthy_aws();
  let k8s = fixtures::healthy_k8s();
  let output = render_playbook(&aws, &k8s).await;

  // The pod IPs section renders with a healthy status when no custom networking issues exist
  assert!(output.contains("sufficient IP space"), "pod IPs section should show healthy status");
  assert!(output.contains("Check [[AWS002]]"), "pod IPs check reference should be present");
}

#[tokio::test]
async fn playbook_deprecation_url_absent_for_1_31_target() {
  // Cluster at 1.30 -> target 1.31 (no deprecation_url in data.yaml)
  let aws = aws_at_version("1.30");
  let k8s = fixtures::healthy_k8s();
  let output = render_playbook(&aws, &k8s).await;

  assert!(!output.contains("API deprecations"), "deprecation link should be absent for 1.31 target");
}

#[tokio::test]
async fn playbook_deprecation_url_present_for_1_32_target() {
  // Cluster at 1.31 -> target 1.32 (has deprecation_url in data.yaml)
  let aws = aws_at_version("1.31");
  let k8s = fixtures::healthy_k8s();
  let output = render_playbook(&aws, &k8s).await;

  assert!(output.contains("API deprecations"), "deprecation link should be present for 1.32 target");
  assert!(output.contains("deprecation-guide/#v1-32"), "deprecation URL should point to 1.32 guide");
}
