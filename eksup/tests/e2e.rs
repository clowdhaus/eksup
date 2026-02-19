mod common;

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
