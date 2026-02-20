# EKS Cluster Insights Integration

**Issue:** #12 - Report on deprecated API usage
**Date:** 2026-02-19

## Goal

Integrate with the EKS cluster insights API (`ListInsights` + `DescribeInsight`) to surface upgrade readiness and misconfiguration findings directly in eksup's analysis output and playbooks.

## Background

AWS EKS provides cluster insights that automatically scan clusters for issues including deprecated Kubernetes API usage, addon compatibility, and hybrid node misconfigurations. These insights are refreshed every 24 hours and use a 30-day rolling window for audit log analysis.

As of 2025, AWS enforces upgrade insights checks as part of cluster upgrades - clusters with ERROR-status insights cannot be upgraded without `--force`.

## Design Decisions

1. **Both categories included**: UPGRADE_READINESS and MISCONFIGURATION insights are both surfaced.
2. **Full detail via DescribeInsight**: After listing insights, each non-PASSING insight is described to get deprecation details, affected resources, and remediation guidance.
3. **Flat finding model**: Each insight becomes one `InsightFinding` row. Consistent with existing patterns.
4. **Two check codes**: EKS009 for upgrade readiness, EKS010 for misconfiguration.
5. **Conservative status mapping**: ERROR=Required, WARNING+UNKNOWN=Recommended, PASSING=skipped.

## Architecture

### New Check Codes

```
EKS009 => { desc: "EKS upgrade readiness insight",        from: None, until: None }
EKS010 => { desc: "EKS cluster misconfiguration insight", from: None, until: None }
```

### Finding Struct

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
```

### Status Mapping

| EKS Status | eksup Remediation | Rationale |
|------------|-------------------|-----------|
| ERROR | Required | AWS blocks upgrades on ERROR insights |
| WARNING | Recommended | Advisory, user should evaluate |
| UNKNOWN | Recommended | Can't evaluate; err on side of caution |
| PASSING | (skip) | No action needed |

### Data Flow

```
ListInsights(cluster_name, filter: statuses=[ERROR, WARNING, UNKNOWN])
  -> for each insight: DescribeInsight(cluster_name, insight_id)
    -> map to InsightFinding:
       - category=UPGRADE_READINESS -> code=EKS009
       - category=MISCONFIGURATION  -> code=EKS010
       - status ERROR -> Remediation::Required
       - status WARNING/UNKNOWN -> Remediation::Recommended
    -> partition into upgrade_readiness / misconfiguration
```

### Files Modified

- `eksup/src/finding.rs` - Add EKS009, EKS010 to `define_codes!`
- `eksup/src/eks/resources.rs` - Add `list_insights()`, `describe_insight()` API wrappers
- `eksup/src/clients.rs` - Add `get_insights()` to `AwsClients` trait
- `eksup/src/eks/checks.rs` - Add `InsightFinding` struct and `cluster_insights()` check function
- `eksup/src/eks/findings.rs` - Add `InsightsFindings` struct, `get_insights_findings()` orchestrator
- `eksup/src/eks/mod.rs` - Export new types
- `eksup/src/analysis.rs` - Add `insights` to `Results`, wire into `filter_recommended` and `to_stdout_table`
- `eksup/src/playbook.rs` - Add insights to `TemplateData`
- `eksup/templates/playbook.md` - Add EKS009/EKS010 sections
- `eksup/tests/common/mock_aws.rs` - Add insights mock data
- `eksup/tests/common/fixtures.rs` - Add insight fixture helpers
- `eksup/tests/integration.rs` - Add insights integration tests

### Findings Container

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct InsightsFindings {
  pub upgrade_readiness: Vec<InsightFinding>,
  pub misconfiguration: Vec<InsightFinding>,
}
```

### Analysis Integration

The insights call is added to the `tokio::try_join!` in `analysis::analyze()` alongside the existing concurrent checks. Results are added to the `Results` struct and rendered in both stdout tables and playbook markdown.
