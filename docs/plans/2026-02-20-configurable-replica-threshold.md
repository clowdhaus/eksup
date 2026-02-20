# Configurable K8S002 Replica Threshold — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a config file (`.eksup.yaml`) supporting per-workload replica threshold overrides for K8S002, change the default minimum from 3 to 2, and retire K8S010.

**Architecture:** New `config.rs` module defines the config structs with serde deserialization. Config is loaded at CLI entrypoint and threaded through `analysis::analyze()` → `k8s::get_kubernetes_findings()` → `K8sFindings::min_replicas()`. The `K8s002Config` provides global default, ignore list, and per-workload overrides.

**Tech Stack:** Rust, serde/serde_yaml for config parsing, clap for CLI args, insta for snapshot tests.

---

### Task 1: Create `config.rs` module with config structs and loading

**Files:**
- Create: `eksup/src/config.rs`
- Modify: `eksup/src/lib.rs:1` (add `pub mod config;`)

**Step 1: Write the config module**

Create `eksup/src/config.rs`:

```rust
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Top-level configuration loaded from `.eksup.yaml`
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Config {
  #[serde(default)]
  pub checks: ChecksConfig,
}

/// Per-check configuration
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChecksConfig {
  #[serde(default, rename = "K8S002")]
  pub k8s002: K8s002Config,
}

/// Configuration for the K8S002 minimum replicas check
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct K8s002Config {
  /// Global minimum replica threshold (default: 2)
  #[serde(default = "default_min_replicas")]
  pub min_replicas: i32,

  /// Workloads to skip entirely for this check
  #[serde(default)]
  pub ignore: Vec<ResourceSelector>,

  /// Per-workload replica threshold overrides
  #[serde(default)]
  pub overrides: Vec<ReplicaOverride>,
}

impl Default for K8s002Config {
  fn default() -> Self {
    Self {
      min_replicas: default_min_replicas(),
      ignore: Vec::new(),
      overrides: Vec::new(),
    }
  }
}

fn default_min_replicas() -> i32 {
  2
}

/// Identifies a Kubernetes workload by name and namespace
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceSelector {
  pub name: String,
  pub namespace: String,
}

/// Per-workload override for the minimum replica threshold
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicaOverride {
  pub name: String,
  pub namespace: String,
  pub min_replicas: i32,
}

impl K8s002Config {
  /// Returns the effective minimum replicas for a given workload.
  /// Returns `None` if the workload is in the ignore list.
  pub fn effective_min_replicas(&self, name: &str, namespace: &str) -> Option<i32> {
    // Check ignore list
    if self.ignore.iter().any(|s| s.name == name && s.namespace == namespace) {
      return None;
    }

    // Check per-workload overrides
    if let Some(ovr) = self.overrides.iter().find(|o| o.name == name && o.namespace == namespace) {
      return Some(ovr.min_replicas);
    }

    // Global default
    Some(self.min_replicas)
  }
}

const DEFAULT_CONFIG_FILE: &str = ".eksup.yaml";

/// Load config from explicit path, `.eksup.yaml` in cwd, or return defaults
pub fn load(path: Option<&str>) -> Result<Config> {
  match path {
    Some(p) => {
      let contents = std::fs::read_to_string(p)
        .with_context(|| format!("Failed to read config file: {p}"))?;
      let config: Config = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse config file: {p}"))?;
      Ok(config)
    }
    None => {
      let default_path = Path::new(DEFAULT_CONFIG_FILE);
      if default_path.exists() {
        let contents = std::fs::read_to_string(default_path)
          .with_context(|| format!("Failed to read config file: {DEFAULT_CONFIG_FILE}"))?;
        let config: Config = serde_yaml::from_str(&contents)
          .with_context(|| format!("Failed to parse config file: {DEFAULT_CONFIG_FILE}"))?;
        Ok(config)
      } else {
        Ok(Config::default())
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_config_has_min_replicas_2() {
    let config = Config::default();
    assert_eq!(config.checks.k8s002.min_replicas, 2);
    assert!(config.checks.k8s002.ignore.is_empty());
    assert!(config.checks.k8s002.overrides.is_empty());
  }

  #[test]
  fn parse_full_config() {
    let yaml = r#"
checks:
  K8S002:
    min_replicas: 3
    ignore:
      - name: singleton
        namespace: batch
    overrides:
      - name: etcd
        namespace: infra
        min_replicas: 5
"#;
    let config: Config = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.checks.k8s002.min_replicas, 3);
    assert_eq!(config.checks.k8s002.ignore.len(), 1);
    assert_eq!(config.checks.k8s002.ignore[0].name, "singleton");
    assert_eq!(config.checks.k8s002.overrides.len(), 1);
    assert_eq!(config.checks.k8s002.overrides[0].min_replicas, 5);
  }

  #[test]
  fn parse_empty_config() {
    let yaml = "{}";
    let config: Config = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.checks.k8s002.min_replicas, 2);
  }

  #[test]
  fn parse_partial_config_uses_defaults() {
    let yaml = r#"
checks:
  K8S002:
    min_replicas: 4
"#;
    let config: Config = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.checks.k8s002.min_replicas, 4);
    assert!(config.checks.k8s002.ignore.is_empty());
    assert!(config.checks.k8s002.overrides.is_empty());
  }

  #[test]
  fn effective_min_replicas_global_default() {
    let config = K8s002Config::default();
    assert_eq!(config.effective_min_replicas("web", "default"), Some(2));
  }

  #[test]
  fn effective_min_replicas_ignored() {
    let config = K8s002Config {
      ignore: vec![ResourceSelector { name: "singleton".into(), namespace: "batch".into() }],
      ..Default::default()
    };
    assert_eq!(config.effective_min_replicas("singleton", "batch"), None);
    assert_eq!(config.effective_min_replicas("other", "batch"), Some(2));
  }

  #[test]
  fn effective_min_replicas_override() {
    let config = K8s002Config {
      overrides: vec![ReplicaOverride { name: "etcd".into(), namespace: "infra".into(), min_replicas: 5 }],
      ..Default::default()
    };
    assert_eq!(config.effective_min_replicas("etcd", "infra"), Some(5));
    assert_eq!(config.effective_min_replicas("other", "default"), Some(2));
  }

  #[test]
  fn effective_min_replicas_ignore_takes_precedence() {
    let config = K8s002Config {
      ignore: vec![ResourceSelector { name: "x".into(), namespace: "ns".into() }],
      overrides: vec![ReplicaOverride { name: "x".into(), namespace: "ns".into(), min_replicas: 5 }],
      ..Default::default()
    };
    assert_eq!(config.effective_min_replicas("x", "ns"), None);
  }

  #[test]
  fn load_missing_file_returns_default() {
    let config = load(None).unwrap();
    assert_eq!(config.checks.k8s002.min_replicas, 2);
  }

  #[test]
  fn load_explicit_missing_file_errors() {
    let result = load(Some("/nonexistent/path.yaml"));
    assert!(result.is_err());
  }
}
```

**Step 2: Register the module in `lib.rs`**

Add `pub mod config;` to the module list at the top of `eksup/src/lib.rs`.

**Step 3: Run tests to verify**

Run: `cargo test -p eksup config::tests`
Expected: All config tests pass.

**Step 4: Commit**

```bash
git add eksup/src/config.rs eksup/src/lib.rs
git commit -m "feat: add config module for check overrides (#57)"
```

---

### Task 2: Wire config into CLI args and entrypoints

**Files:**
- Modify: `eksup/src/lib.rs:69-96` (add `--config` to Analysis)
- Modify: `eksup/src/lib.rs:112-137` (add `--config` to Playbook)
- Modify: `eksup/src/lib.rs:147-184` (load config in `analyze()`)
- Modify: `eksup/src/lib.rs:220-264` (load config in `create()`)

**Step 1: Add `--config` CLI flag**

In `eksup/src/lib.rs`, add to the `Analysis` struct after the `ignore_recommended` field:

```rust
  /// Path to config file (defaults to .eksup.yaml in current directory)
  #[arg(long)]
  pub config: Option<String>,
```

Add the same field to the `Playbook` struct.

**Step 2: Load config in `analyze()` function**

In the `analyze()` function, after loading AWS config and before analysis, add:

```rust
  let config = config::load(args.config.as_deref())?;
```

Pass `&config` to `analysis::analyze()`.

**Step 3: Load config in `create()` function**

Same pattern — load config and pass to `analysis::analyze()`.

**Step 4: Run `cargo check -p eksup`**

This will fail because `analysis::analyze()` doesn't accept `&Config` yet — that's expected and will be fixed in Task 3.

**Step 5: Commit**

```bash
git add eksup/src/lib.rs
git commit -m "feat: add --config CLI flag to analyze and create commands"
```

---

### Task 3: Thread config through analysis pipeline

**Files:**
- Modify: `eksup/src/analysis.rs:86-91` (add `&Config` parameter)
- Modify: `eksup/src/k8s/findings.rs:25-29` (add `&K8s002Config` parameter)
- Modify: `eksup/src/k8s/findings.rs:36` (pass config to min_replicas check)

**Step 1: Update `analysis::analyze()` signature**

Change `eksup/src/analysis.rs` line 86:

```rust
pub async fn analyze(
  aws: &impl AwsClients,
  k8s: &impl K8sClients,
  cluster: &Cluster,
  target_minor: i32,
  config: &crate::config::Config,
) -> Result<Results> {
```

Update the `k8s::get_kubernetes_findings` call to pass config:

```rust
    k8s::get_kubernetes_findings(k8s, control_plane_minor, target_minor, &config.checks.k8s002),
```

**Step 2: Update `k8s::get_kubernetes_findings()` signature**

Change `eksup/src/k8s/findings.rs` line 25:

```rust
pub async fn get_kubernetes_findings(
  k8s: &impl K8sClients,
  control_plane_minor: i32,
  target_minor: i32,
  k8s002_config: &crate::config::K8s002Config,
) -> Result<KubernetesFindings> {
```

Update the min_replicas collection line (line 36):

```rust
  let min_replicas: Vec<checks::MinReplicas> = resources.iter().filter_map(|s| s.min_replicas(k8s002_config)).collect();
```

**Step 3: Update callers in `lib.rs`**

Pass `&config` to both `analysis::analyze()` calls.

**Step 4: Run `cargo check -p eksup`**

Expected: Fails because `K8sFindings::min_replicas()` trait doesn't accept config yet — fixed in Task 4.

**Step 5: Commit**

```bash
git add eksup/src/analysis.rs eksup/src/k8s/findings.rs eksup/src/lib.rs
git commit -m "feat: thread config through analysis pipeline"
```

---

### Task 4: Update K8S002 check implementation

**Files:**
- Modify: `eksup/src/k8s/checks.rs:495-499` (trait method signature)
- Modify: `eksup/src/k8s/checks.rs:146` (success message)
- Modify: `eksup/src/k8s/resources.rs:425-446` (implementation)

**Step 1: Update the `K8sFindings` trait**

In `eksup/src/k8s/checks.rs` line 498, change:

```rust
  /// K8S002 - check if resources meet configured minimum replicas
  fn min_replicas(&self, config: &crate::config::K8s002Config) -> Option<MinReplicas>;
```

**Step 2: Update the success message**

In `eksup/src/k8s/checks.rs` line 146, change:

```rust
finding::impl_findings!(MinReplicas, "✅ - All relevant Kubernetes workloads meet the configured minimum replicas");
```

**Step 3: Update `StdResource::min_replicas()` implementation**

In `eksup/src/k8s/resources.rs` lines 425-446, replace the entire method:

```rust
  fn min_replicas(&self, config: &crate::config::K8s002Config) -> Option<checks::MinReplicas> {
    let replicas = self.spec.replicas;

    match replicas {
      Some(replicas) => {
        if replicas <= 0 {
          return None;
        }

        let threshold = config.effective_min_replicas(
          &self.metadata.name,
          &self.metadata.namespace,
        )?;

        if replicas < threshold {
          Some(checks::MinReplicas {
            finding: Finding::new(Code::K8S002, Remediation::Required),
            resource: self.get_resource(),
            replicas,
          })
        } else {
          None
        }
      }
      None => None,
    }
  }
```

Note: The old CoreDNS special case and hardcoded `< 3` are both removed.

**Step 4: Run `cargo check -p eksup`**

Expected: Compiles successfully.

**Step 5: Run `cargo test -p eksup`**

Expected: Unit tests pass; snapshot tests will need updating (Task 6).

**Step 6: Commit**

```bash
git add eksup/src/k8s/checks.rs eksup/src/k8s/resources.rs
git commit -m "feat: implement configurable K8S002 threshold with ignore/overrides"
```

---

### Task 5: Update integration and e2e tests

**Files:**
- Modify: `eksup/tests/integration.rs` (update all `analyze()` calls)
- Modify: `eksup/tests/e2e.rs` (update `run_analysis()` helper)
- Modify: `eksup/tests/common/fixtures.rs` (no changes expected)

**Step 1: Update `integration.rs` analyze calls**

Every call to `eksup::analysis::analyze()` needs a `&Config::default()` argument. Add at the top:

```rust
use eksup::config::Config;
```

Update each `analyze()` call, e.g. line 214:

```rust
  let results = eksup::analysis::analyze(&aws, &k8s, &aws.cluster, 31, &Config::default()).await.unwrap();
```

Same for all other `analyze()` calls in the file.

Update `k8s::get_kubernetes_findings` calls to pass `&K8s002Config::default()`:

```rust
use eksup::config::K8s002Config;

// e.g. line 138:
let result = eksup::k8s::get_kubernetes_findings(&k8s, 30, 31, &K8s002Config::default()).await.unwrap();
```

**Step 2: Update `e2e.rs` helpers**

Update the `run_analysis()` helper function:

```rust
use eksup::config::Config;

async fn run_analysis(aws: &MockAwsClients, k8s: &MockK8sClients) -> Results {
  let cluster_version = aws.cluster.version().unwrap();
  let target_minor = eksup::version::get_target_version(cluster_version).unwrap();
  eksup::analysis::analyze(aws, k8s, &aws.cluster, target_minor, &Config::default()).await.unwrap()
}
```

**Step 3: Run `cargo test -p eksup`**

Expected: Tests compile. Some snapshot tests may fail due to the threshold change from 3 → 2 (e.g., the `workload_issues` tests use `make_deployment("api", "backend", 2)` which will no longer be flagged).

**Step 4: Update snapshots**

Run: `cargo insta test -p eksup --review` to update snapshot files.

Review each changed snapshot:
- `workload_issues_text.snap` — "api" deployment with 2 replicas should no longer appear
- `workload_issues_json.snap` — same
- `filter_recommended_text.snap` — may change
- `mixed_findings_text.snap` / `mixed_findings_json.snap` — may change
- `playbook_mixed.snap` — may change
- Playbook snapshots with success messages will show new wording

Accept the updated snapshots: `cargo insta review`

**Step 5: Commit**

```bash
git add eksup/tests/ eksup/src/
git commit -m "test: update integration and e2e tests for config parameter"
```

---

### Task 6: Add K8S002 config-specific tests

**Files:**
- Modify: `eksup/tests/integration.rs` (add new test cases)

**Step 1: Add integration test for config override**

```rust
#[tokio::test]
async fn kubernetes_findings_min_replicas_with_override() {
  let k8s = MockK8sClients {
    resources: vec![fixtures::make_deployment("web", "default", 2)],
    ..Default::default()
  };

  // With default config (min_replicas: 2), 2 replicas should NOT trigger
  let result = eksup::k8s::get_kubernetes_findings(&k8s, 30, 31, &K8s002Config::default()).await.unwrap();
  assert!(result.min_replicas.is_empty(), "2 replicas should pass with default threshold of 2");

  // With min_replicas: 3, 2 replicas SHOULD trigger
  let strict_config = K8s002Config { min_replicas: 3, ..Default::default() };
  let result = eksup::k8s::get_kubernetes_findings(&k8s, 30, 31, &strict_config).await.unwrap();
  assert_eq!(result.min_replicas.len(), 1, "2 replicas should fail with threshold of 3");
}
```

**Step 2: Add integration test for ignore list**

```rust
#[tokio::test]
async fn kubernetes_findings_min_replicas_ignored() {
  let k8s = MockK8sClients {
    resources: vec![fixtures::make_deployment("singleton", "batch", 1)],
    ..Default::default()
  };

  let config = K8s002Config {
    ignore: vec![eksup::config::ResourceSelector {
      name: "singleton".into(),
      namespace: "batch".into(),
    }],
    ..Default::default()
  };

  let result = eksup::k8s::get_kubernetes_findings(&k8s, 30, 31, &config).await.unwrap();
  assert!(result.min_replicas.is_empty(), "ignored workload should not trigger finding");
}
```

**Step 3: Add integration test for per-workload override**

```rust
#[tokio::test]
async fn kubernetes_findings_min_replicas_per_workload_override() {
  let k8s = MockK8sClients {
    resources: vec![
      fixtures::make_deployment("etcd", "infra", 3),
      fixtures::make_deployment("web", "default", 2),
    ],
    ..Default::default()
  };

  let config = K8s002Config {
    overrides: vec![eksup::config::ReplicaOverride {
      name: "etcd".into(),
      namespace: "infra".into(),
      min_replicas: 5,
    }],
    ..Default::default()
  };

  let result = eksup::k8s::get_kubernetes_findings(&k8s, 30, 31, &config).await.unwrap();
  // etcd has 3 but needs 5 -> triggers
  // web has 2 and needs 2 (default) -> passes
  assert_eq!(result.min_replicas.len(), 1);
  assert_eq!(result.min_replicas[0].resource.name, "etcd");
}
```

**Step 4: Run tests**

Run: `cargo test -p eksup`
Expected: All tests pass.

**Step 5: Commit**

```bash
git add eksup/tests/integration.rs
git commit -m "test: add K8S002 config override integration tests"
```

---

### Task 7: Retire K8S010 and update K8S002 description

**Files:**
- Modify: `eksup/src/finding.rs:173` (retire K8S010)
- Modify: `eksup/src/finding.rs:165` (update K8S002 description)

**Step 1: Mark K8S010 as retired**

In `eksup/src/finding.rs` line 173, change:

```rust
  K8S010 => { desc: "EBS CSI driver not installed",  from: None, until: Some(24) },
```

**Step 2: Update K8S002 description**

In `eksup/src/finding.rs` line 165, change:

```rust
  K8S002 => { desc: "Insufficient number of .spec.replicas (configurable)",  from: None, until: None },
```

**Step 3: Update the `always_relevant_codes_apply_to_any_version` test**

The test at line 239 references K8S002 in the "always" array — it should still pass since K8S002 remains always-applicable. No change needed.

**Step 4: Add a test for K8S010 retirement**

```rust
  #[test]
  fn k8s010_is_retired() {
    assert!(Code::K8S010.is_retired());
  }
```

**Step 5: Run tests**

Run: `cargo test -p eksup finding::tests`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add eksup/src/finding.rs
git commit -m "chore: retire K8S010 (EBS CSI driver) and update K8S002 description"
```

---

### Task 8: Final verification and snapshot cleanup

**Step 1: Run full test suite**

Run: `cargo test -p eksup`
Expected: All tests pass.

**Step 2: Run clippy**

Run: `cargo clippy -p eksup -- -D warnings`
Expected: No warnings.

**Step 3: Review all snapshot changes**

Run: `cargo insta test -p eksup` then `cargo insta review`
Verify each snapshot diff makes sense (2-replica workloads no longer flagged, new success message wording).

**Step 4: Final commit if any cleanup needed**

```bash
git add -A
git commit -m "chore: final cleanup and snapshot updates"
```
