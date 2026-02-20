# EKS Cluster Insights Integration

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate the EKS cluster insights API (ListInsights + DescribeInsight) to surface upgrade readiness and misconfiguration findings in eksup's analysis and playbooks, addressing issue #12.

**Architecture:** Add a new `InsightsFindings` category to the analysis pipeline. The data fetching layer calls `ListInsights` (filtered to non-PASSING statuses) then `DescribeInsight` for each, maps them to `InsightFinding` structs partitioned by category (UPGRADE_READINESS → EKS009, MISCONFIGURATION → EKS010), and wires them through the existing finding/table/playbook rendering pipeline.

**Tech Stack:** Rust, aws-sdk-eks (list_insights/describe_insight), serde, tabled, handlebars

---

## Task 1: Add EKS009 and EKS010 check codes

Add two new check codes to the finding system for cluster insights.

**Files:**
- Modify: `eksup/src/finding.rs`

### Step 1: Add the new codes to `define_codes!`

In `eksup/src/finding.rs`, add two new entries to the `define_codes!` macro invocation, after the `EKS008` line:

```rust
  EKS009 => { desc: "EKS upgrade readiness insight",        from: None, until: None },
  EKS010 => { desc: "EKS cluster misconfiguration insight", from: None, until: None },
```

### Step 2: Run tests to verify codes compile

```bash
cargo test -p eksup -- finding::tests
```

Expected: All existing finding tests pass, no compile errors.

### Step 3: Commit

```bash
git add eksup/src/finding.rs
git commit -m "feat: add EKS009/EKS010 check codes for cluster insights"
```

---

## Task 2: Add insights data types and API wrappers

Add the SDK wrapper functions for `list_insights` and `describe_insight` to the resources module.

**Files:**
- Modify: `eksup/src/eks/resources.rs`

### Step 1: Add the insight data types

Add these types at the bottom of `eksup/src/eks/resources.rs`, before any `#[cfg(test)]` block:

```rust
/// Simplified representation of an EKS cluster insight
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClusterInsight {
  pub id: String,
  pub name: String,
  pub category: String,
  pub status: String,
  pub status_reason: String,
  pub kubernetes_version: String,
  pub description: String,
  pub recommendation: String,
}
```

### Step 2: Add the `list_insights` function

Add this function after the `ClusterInsight` struct:

```rust
/// List all cluster insights that are not in a PASSING state
pub async fn list_insights(client: &EksClient, cluster_name: &str) -> Result<Vec<String>> {
  use aws_sdk_eks::types::{InsightStatusValue, InsightsFilter};

  let filter = InsightsFilter::builder()
    .statuses(InsightStatusValue::Error)
    .statuses(InsightStatusValue::Warning)
    .statuses(InsightStatusValue::Unknown)
    .build();

  let mut insight_ids = Vec::new();
  let mut next_token: Option<String> = None;
  loop {
    let mut req = client
      .list_insights()
      .cluster_name(cluster_name)
      .filter(filter.clone());
    if let Some(token) = &next_token {
      req = req.next_token(token);
    }
    let resp = req.send().await.context("Failed to list cluster insights")?;

    for summary in resp.insights() {
      if let Some(id) = summary.id() {
        insight_ids.push(id.to_string());
      }
    }

    next_token = resp.next_token;
    if next_token.is_none() {
      break;
    }
  }

  Ok(insight_ids)
}
```

### Step 3: Add the `describe_insight` function

Add this function after `list_insights`:

```rust
/// Describe a single cluster insight by ID
pub async fn describe_insight(
  client: &EksClient,
  cluster_name: &str,
  insight_id: &str,
) -> Result<ClusterInsight> {
  let resp = client
    .describe_insight()
    .cluster_name(cluster_name)
    .id(insight_id)
    .send()
    .await
    .context(format!("Failed to describe insight '{insight_id}'"))?;

  let insight = resp.insight.context("No insight found in response")?;

  let (status, status_reason) = match insight.insight_status() {
    Some(s) => (
      s.status().map(|v| v.as_str().to_string()).unwrap_or_default(),
      s.reason().unwrap_or_default().to_string(),
    ),
    None => (String::new(), String::new()),
  };

  Ok(ClusterInsight {
    id: insight.id().unwrap_or_default().to_string(),
    name: insight.name().unwrap_or_default().to_string(),
    category: insight.category().map(|c| c.as_str().to_string()).unwrap_or_default(),
    status,
    status_reason,
    kubernetes_version: insight.kubernetes_version().unwrap_or_default().to_string(),
    description: insight.description().unwrap_or_default().to_string(),
    recommendation: insight.recommendation().unwrap_or_default().to_string(),
  })
}
```

### Step 4: Add the combined `get_cluster_insights` function

Add this convenience function after `describe_insight`:

```rust
/// Fetch all non-PASSING cluster insights with full details
pub async fn get_cluster_insights(
  client: &EksClient,
  cluster_name: &str,
) -> Result<Vec<ClusterInsight>> {
  let insight_ids = list_insights(client, cluster_name).await?;

  let mut insights = Vec::new();
  for id in &insight_ids {
    let insight = describe_insight(client, cluster_name, id).await?;
    insights.push(insight);
  }

  Ok(insights)
}
```

### Step 5: Verify it compiles

```bash
cargo check -p eksup
```

Expected: Compiles without errors.

### Step 6: Commit

```bash
git add eksup/src/eks/resources.rs
git commit -m "feat: add EKS insights API wrapper functions"
```

---

## Task 3: Add insights to the AwsClients trait and mocks

Wire the insights data fetching through the client trait abstraction.

**Files:**
- Modify: `eksup/src/clients.rs`
- Modify: `eksup/tests/common/mock_aws.rs`
- Modify: `eksup/tests/common/fixtures.rs`

### Step 1: Add trait method to `AwsClients`

In `eksup/src/clients.rs`, add this import at the top alongside the existing resource imports:

```rust
use crate::eks::resources::ClusterInsight;
```

Then add this method to the `AwsClients` trait, after the `get_ebs_volume_storage` method:

```rust
fn get_cluster_insights(&self, cluster_name: &str) -> impl std::future::Future<Output = Result<Vec<ClusterInsight>>> + Send;
```

### Step 2: Implement on `RealAwsClients`

In the `impl AwsClients for RealAwsClients` block, add:

```rust
async fn get_cluster_insights(&self, cluster_name: &str) -> Result<Vec<ClusterInsight>> {
  eks_resources::get_cluster_insights(&self.eks, cluster_name).await
}
```

### Step 3: Add mock field and implementation

In `eksup/tests/common/mock_aws.rs`, add the import:

```rust
use eksup::eks::resources::ClusterInsight;
```

Add a new field to `MockAwsClients`:

```rust
pub insights: Vec<ClusterInsight>,
```

In the `Default` impl, add:

```rust
insights: vec![],
```

In the `impl AwsClients for MockAwsClients` block, add:

```rust
async fn get_cluster_insights(&self, _cluster_name: &str) -> Result<Vec<ClusterInsight>> {
  Ok(self.insights.clone())
}
```

In the `impl AwsClients for MockAwsClientsError` block, add:

```rust
async fn get_cluster_insights(&self, _cluster_name: &str) -> Result<Vec<ClusterInsight>> { bail!("mock AWS error") }
```

### Step 4: Add insight fixture helper

In `eksup/tests/common/fixtures.rs`, add the import:

```rust
use eksup::eks::resources::ClusterInsight;
```

Then add this helper function:

```rust
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
```

### Step 5: Verify it compiles

```bash
cargo test -p eksup --no-run
```

Expected: Compiles without errors.

### Step 6: Commit

```bash
git add eksup/src/clients.rs eksup/tests/common/mock_aws.rs eksup/tests/common/fixtures.rs
git commit -m "feat: add cluster insights to AwsClients trait and mocks"
```

---

## Task 4: Add InsightFinding struct and check function

Add the finding struct and the pure check function that maps raw insights to findings.

**Files:**
- Modify: `eksup/src/eks/checks.rs`

### Step 1: Add the `InsightFinding` struct

In `eksup/src/eks/checks.rs`, add these imports to the top `use` block if not already present (the `resources` import is already there):

No new imports needed - `finding`, `Code`, `Finding`, `Remediation`, `resources`, `Serialize`, `Deserialize`, and `Tabled` are all already imported.

Add the struct after the `ServiceLimitFinding` struct and its `impl_findings!` call:

```rust
#[derive(Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct InsightFinding {
  #[tabled(inline)]
  pub finding: finding::Finding,
  #[tabled(rename = "INSIGHT")]
  pub name: String,
  #[tabled(rename = "STATUS")]
  pub status: String,
  #[tabled(rename = "VERSION")]
  pub kubernetes_version: String,
  #[tabled(rename = "DESCRIPTION")]
  pub description: String,
  #[tabled(rename = "RECOMMENDATION")]
  pub recommendation: String,
}

finding::impl_findings!(InsightFinding, "✅ - No cluster insight issues found");
```

### Step 2: Add the check function

Add this function after the struct:

```rust
/// Map raw EKS cluster insights to findings, partitioned by category
///
/// Returns (upgrade_readiness, misconfiguration) tuples.
/// PASSING insights are pre-filtered by the API call; this function
/// maps ERROR → Required and WARNING/UNKNOWN → Recommended.
pub(crate) fn cluster_insights(
  insights: &[resources::ClusterInsight],
) -> (Vec<InsightFinding>, Vec<InsightFinding>) {
  let mut upgrade_readiness = Vec::new();
  let mut misconfiguration = Vec::new();

  for insight in insights {
    let remediation = match insight.status.as_str() {
      "ERROR" => Remediation::Required,
      "WARNING" | "UNKNOWN" => Remediation::Recommended,
      _ => continue,
    };

    let code = if insight.category == "UPGRADE_READINESS" {
      Code::EKS009
    } else {
      Code::EKS010
    };

    let finding = InsightFinding {
      finding: Finding::new(code, remediation),
      name: insight.name.clone(),
      status: insight.status.clone(),
      kubernetes_version: insight.kubernetes_version.clone(),
      description: insight.description.clone(),
      recommendation: insight.recommendation.clone(),
    };

    if insight.category == "UPGRADE_READINESS" {
      upgrade_readiness.push(finding);
    } else {
      misconfiguration.push(finding);
    }
  }

  (upgrade_readiness, misconfiguration)
}
```

### Step 3: Add unit tests

Add these tests inside the existing `#[cfg(test)] mod tests` block in `eks/checks.rs`:

```rust
  use crate::eks::resources::ClusterInsight;

  fn make_test_insight(category: &str, status: &str) -> ClusterInsight {
    ClusterInsight {
      id: "test-id".into(),
      name: "Test Insight".into(),
      category: category.into(),
      status: status.into(),
      status_reason: String::new(),
      kubernetes_version: "1.31".into(),
      description: "Test description".into(),
      recommendation: "Test recommendation".into(),
    }
  }

  // ---------- cluster_insights ----------

  #[test]
  fn cluster_insights_empty() {
    let (upgrade, misconfig) = cluster_insights(&[]);
    assert!(upgrade.is_empty());
    assert!(misconfig.is_empty());
  }

  #[test]
  fn cluster_insights_error_is_required() {
    let insight = make_test_insight("UPGRADE_READINESS", "ERROR");
    let (upgrade, _) = cluster_insights(&[insight]);
    assert_eq!(upgrade.len(), 1);
    assert!(matches!(upgrade[0].finding.remediation, Remediation::Required));
  }

  #[test]
  fn cluster_insights_warning_is_recommended() {
    let insight = make_test_insight("UPGRADE_READINESS", "WARNING");
    let (upgrade, _) = cluster_insights(&[insight]);
    assert_eq!(upgrade.len(), 1);
    assert!(matches!(upgrade[0].finding.remediation, Remediation::Recommended));
  }

  #[test]
  fn cluster_insights_unknown_is_recommended() {
    let insight = make_test_insight("MISCONFIGURATION", "UNKNOWN");
    let (_, misconfig) = cluster_insights(&[insight]);
    assert_eq!(misconfig.len(), 1);
    assert!(matches!(misconfig[0].finding.remediation, Remediation::Recommended));
  }

  #[test]
  fn cluster_insights_passing_skipped() {
    let insight = make_test_insight("UPGRADE_READINESS", "PASSING");
    let (upgrade, misconfig) = cluster_insights(&[insight]);
    assert!(upgrade.is_empty());
    assert!(misconfig.is_empty());
  }

  #[test]
  fn cluster_insights_partitions_by_category() {
    let insights = vec![
      make_test_insight("UPGRADE_READINESS", "ERROR"),
      make_test_insight("MISCONFIGURATION", "WARNING"),
      make_test_insight("UPGRADE_READINESS", "WARNING"),
    ];
    let (upgrade, misconfig) = cluster_insights(&insights);
    assert_eq!(upgrade.len(), 2);
    assert_eq!(misconfig.len(), 1);
  }

  #[test]
  fn cluster_insights_code_mapping() {
    let insights = vec![
      make_test_insight("UPGRADE_READINESS", "ERROR"),
      make_test_insight("MISCONFIGURATION", "ERROR"),
    ];
    let (upgrade, misconfig) = cluster_insights(&insights);
    assert_eq!(upgrade[0].finding.code.to_string(), "EKS009");
    assert_eq!(misconfig[0].finding.code.to_string(), "EKS010");
  }
```

### Step 4: Run the tests

```bash
cargo test -p eksup -- eks::checks::tests::cluster_insights
```

Expected: All 6 cluster_insights tests pass.

### Step 5: Commit

```bash
git add eksup/src/eks/checks.rs
git commit -m "feat: add InsightFinding struct and cluster_insights check"
```

---

## Task 5: Wire insights into the findings pipeline

Add the `InsightsFindings` orchestrator and wire it through the analysis pipeline.

**Files:**
- Modify: `eksup/src/eks/findings.rs`
- Modify: `eksup/src/eks/mod.rs`
- Modify: `eksup/src/analysis.rs`

### Step 1: Add `InsightsFindings` struct to `eks/findings.rs`

At the bottom of `eksup/src/eks/findings.rs`, add:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct InsightsFindings {
  pub upgrade_readiness: Vec<checks::InsightFinding>,
  pub misconfiguration: Vec<checks::InsightFinding>,
}

pub async fn get_insights_findings(
  aws: &impl AwsClients,
  cluster_name: &str,
) -> Result<InsightsFindings> {
  let insights = aws.get_cluster_insights(cluster_name).await?;
  let (upgrade_readiness, misconfiguration) = checks::cluster_insights(&insights);

  Ok(InsightsFindings {
    upgrade_readiness,
    misconfiguration,
  })
}
```

### Step 2: Export from `eks/mod.rs`

In `eksup/src/eks/mod.rs`, update the `pub use` statement to include the new types:

```rust
pub use findings::{
  AddonFindings, ClusterFindings, DataPlaneFindings, InsightsFindings, ServiceLimitFindings,
  SubnetFindings, get_addon_findings, get_cluster_findings, get_data_plane_findings,
  get_insights_findings, get_service_limit_findings, get_subnet_findings,
};
```

### Step 3: Add `insights` to `Results` struct

In `eksup/src/analysis.rs`, add the `insights` field to the `Results` struct:

```rust
pub struct Results {
  pub cluster: eks::ClusterFindings,
  pub subnets: eks::SubnetFindings,
  pub data_plane: eks::DataPlaneFindings,
  pub addons: eks::AddonFindings,
  pub kubernetes: k8s::KubernetesFindings,
  pub service_limits: eks::ServiceLimitFindings,
  pub insights: eks::InsightsFindings,
}
```

### Step 4: Add insights to `filter_recommended`

In the `filter_recommended` method, add these lines after the service_limits lines:

```rust
self.insights.upgrade_readiness.retain(|f| !f.finding.remediation.is_recommended());
self.insights.misconfiguration.retain(|f| !f.finding.remediation.is_recommended());
```

### Step 5: Add insights to `to_stdout_table`

In the `to_stdout_table` method, add these lines after the service_limits lines:

```rust
output.push_str(&self.insights.upgrade_readiness.to_stdout_table()?);
output.push_str(&self.insights.misconfiguration.to_stdout_table()?);
```

### Step 6: Wire into the `analyze` function

In the `analyze` function, update the `tokio::try_join!` call to include insights:

```rust
let (subnet_findings, addon_findings, dataplane_findings, kubernetes_findings, service_limit_findings, insights_findings) = tokio::try_join!(
  eks::get_subnet_findings(aws, k8s, cluster),
  eks::get_addon_findings(aws, cluster_name, cluster_version, target_minor),
  eks::get_data_plane_findings(aws, cluster, target_minor),
  k8s::get_kubernetes_findings(k8s, control_plane_minor, target_minor),
  eks::get_service_limit_findings(aws),
  eks::get_insights_findings(aws, cluster_name),
)?;
```

And update the `Results` construction to include insights:

```rust
Ok(Results {
  cluster: cluster_findings,
  subnets: subnet_findings,
  addons: addon_findings,
  data_plane: dataplane_findings,
  kubernetes: kubernetes_findings,
  service_limits: service_limit_findings,
  insights: insights_findings,
})
```

### Step 7: Verify it compiles

```bash
cargo check -p eksup
```

Expected: Compiles (tests may not compile yet due to mock updates needed - that's OK).

### Step 8: Commit

```bash
git add eksup/src/eks/findings.rs eksup/src/eks/mod.rs eksup/src/analysis.rs
git commit -m "feat: wire cluster insights into analysis pipeline"
```

---

## Task 6: Wire insights into playbook rendering

Add insights findings to the playbook template data and template.

**Files:**
- Modify: `eksup/src/playbook.rs`
- Modify: `eksup/templates/playbook.md`

### Step 1: Add insights fields to `TemplateData`

In `eksup/src/playbook.rs`, add these two fields to the `TemplateData` struct, after `ebs_gp3_limits`:

```rust
  upgrade_readiness_insights: String,
  misconfiguration_insights: String,
```

### Step 2: Populate the new fields in `render()`

In the `render` function, add these lines to the `TemplateData` construction, after the `ebs_gp3_limits` line:

```rust
    upgrade_readiness_insights: analysis.insights.upgrade_readiness.to_markdown_table("\t")?,
    misconfiguration_insights: analysis.insights.misconfiguration.to_markdown_table("\t")?,
```

Note: `analysis.insights` must be accessed before `analysis` is partially moved. Since `analysis.insights` uses `to_markdown_table` which takes `&self`, this should work if placed after the existing `analysis.service_limits.*` lines (which also use references).

### Step 3: Add insights sections to the playbook template

In `eksup/templates/playbook.md`, add a new section after the "Control Plane Pre-Upgrade" item 5 (the deprecated API check section, before `### Control Plane Upgrade`). Insert before the line `### Control Plane Upgrade`:

```
6. Review [EKS cluster insights](https://docs.aws.amazon.com/eks/latest/userguide/cluster-insights.html) for upgrade readiness issues. Amazon EKS automatically scans clusters against potential upgrade-impacting issues including deprecated Kubernetes API usage. Clusters with ERROR-status insights cannot be upgraded without the `--force` flag.

    <details>
    <summary>📌 CLI Example</summary>

    ```sh
    aws eks list-insights --region {{ region }} --cluster-name {{ cluster_name }}
    ```

    </details>

    #### Check [[EKS009]](https://clowdhaus.github.io/eksup/info/checks/#eks009)
{{ upgrade_readiness_insights }}

    #### Check [[EKS010]](https://clowdhaus.github.io/eksup/info/checks/#eks010)
{{ misconfiguration_insights }}
```

### Step 4: Verify it compiles

```bash
cargo check -p eksup
```

Expected: Compiles without errors.

### Step 5: Commit

```bash
git add eksup/src/playbook.rs eksup/templates/playbook.md
git commit -m "feat: add cluster insights to playbook template"
```

---

## Task 7: Add integration tests and update snapshots

Add integration tests for the insights pipeline and update any affected snapshot tests.

**Files:**
- Modify: `eksup/tests/integration.rs`

### Step 1: Add insights integration tests

Add these tests to `eksup/tests/integration.rs`:

```rust
// ============================================================================
// Insights findings
// ============================================================================

#[tokio::test]
async fn insights_findings_empty() {
  let aws = fixtures::healthy_aws();
  let result = eksup::eks::get_insights_findings(&aws, "test-cluster").await.unwrap();
  assert!(result.upgrade_readiness.is_empty());
  assert!(result.misconfiguration.is_empty());
}

#[tokio::test]
async fn insights_findings_with_issues() {
  let mut aws = fixtures::healthy_aws();
  aws.insights = vec![
    fixtures::make_insight(
      "insight-1",
      "Deprecated API usage",
      "UPGRADE_READINESS",
      "ERROR",
      "1.31",
      "APIs removed in v1.32",
      "Update to stable APIs",
    ),
    fixtures::make_insight(
      "insight-2",
      "Hybrid node config",
      "MISCONFIGURATION",
      "WARNING",
      "",
      "Node configuration issue",
      "Check hybrid node setup",
    ),
  ];

  let result = eksup::eks::get_insights_findings(&aws, "test-cluster").await.unwrap();
  assert_eq!(result.upgrade_readiness.len(), 1, "should have 1 upgrade readiness insight");
  assert_eq!(result.misconfiguration.len(), 1, "should have 1 misconfiguration insight");
}

#[tokio::test]
async fn insights_findings_status_mapping() {
  let mut aws = fixtures::healthy_aws();
  aws.insights = vec![
    fixtures::make_insight("id-1", "Error insight", "UPGRADE_READINESS", "ERROR", "1.31", "desc", "rec"),
    fixtures::make_insight("id-2", "Warning insight", "UPGRADE_READINESS", "WARNING", "1.31", "desc", "rec"),
    fixtures::make_insight("id-3", "Unknown insight", "UPGRADE_READINESS", "UNKNOWN", "1.31", "desc", "rec"),
  ];

  let result = eksup::eks::get_insights_findings(&aws, "test-cluster").await.unwrap();
  assert_eq!(result.upgrade_readiness.len(), 3);

  use eksup::finding::Remediation;
  assert!(matches!(result.upgrade_readiness[0].finding.remediation, Remediation::Required));
  assert!(matches!(result.upgrade_readiness[1].finding.remediation, Remediation::Recommended));
  assert!(matches!(result.upgrade_readiness[2].finding.remediation, Remediation::Recommended));
}
```

### Step 2: Run all tests

```bash
cargo test -p eksup
```

Expected: All tests pass. If snapshot tests fail due to the new `insights` field in `Results`, update them with:

```bash
cargo insta review
```

or

```bash
INSTA_UPDATE=1 cargo test -p eksup
```

### Step 3: Commit

```bash
git add eksup/tests/integration.rs
git commit -m "feat: add integration tests for cluster insights"
```

If snapshot files were updated:

```bash
git add eksup/tests/snapshots/
git commit -m "test: update snapshots for cluster insights"
```

---

## Task 8: Final verification

Run the full test suite, clippy, and build.

### Step 1: Run all tests

```bash
cargo test -p eksup
```

Expected: All tests pass.

### Step 2: Run clippy

```bash
cargo clippy -p eksup -- -D warnings
```

Expected: No warnings.

### Step 3: Build release

```bash
cargo build -p eksup
```

Expected: Builds successfully.

### Step 4: Verify the new checks appear in the output structure

```bash
cargo test -p eksup -- analyze_healthy_cluster
```

Expected: Passes with the new `insights` field in `Results`.
