use std::{collections::HashMap, sync::OnceLock};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::finding::Code;

/// Top-level configuration loaded from `.eksup.yaml` or an explicit path.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Config {
  #[serde(default)]
  pub checks: ChecksConfig,
}

/// Per-check configuration knobs.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChecksConfig {
  #[serde(default)]
  pub all: WorkloadCheckConfig,

  #[serde(default, rename = "K8S002")]
  pub k8s002: K8s002Config,

  #[serde(default, rename = "K8S003")]
  pub k8s003: WorkloadCheckConfig,

  #[serde(default, rename = "K8S004")]
  pub k8s004: WorkloadCheckConfig,

  #[serde(default, rename = "K8S005")]
  pub k8s005: WorkloadCheckConfig,

  #[serde(default, rename = "K8S006")]
  pub k8s006: WorkloadCheckConfig,

  #[serde(default, rename = "K8S007")]
  pub k8s007: WorkloadCheckConfig,

  #[serde(default, rename = "K8S008")]
  pub k8s008: WorkloadCheckConfig,

  #[serde(default, rename = "K8S013")]
  pub k8s013: WorkloadCheckConfig,

  // K8S012 (KubeProxyIpvsMode) is intentionally absent — its finding type has
  // no `resource` field, so name+namespace ignore doesn't apply.
  #[serde(skip)]
  compiled: OnceLock<CompiledChecks>,
}

// Manual Clone because OnceLock doesn't derive Clone. Cache resets; the clone
// rebuilds on first compiled() call.
impl Clone for ChecksConfig {
  fn clone(&self) -> Self {
    Self {
      all: self.all.clone(),
      k8s002: self.k8s002.clone(),
      k8s003: self.k8s003.clone(),
      k8s004: self.k8s004.clone(),
      k8s005: self.k8s005.clone(),
      k8s006: self.k8s006.clone(),
      k8s007: self.k8s007.clone(),
      k8s008: self.k8s008.clone(),
      k8s013: self.k8s013.clone(),
      compiled: OnceLock::new(),
    }
  }
}

/// Shared shape for every workload check that only needs ignore rules.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadCheckConfig {
  #[serde(default)]
  pub ignore: Vec<ResourceSelector>,
}

/// A `ResourceSelector` with its name/namespace patterns compiled into globset matchers.
#[derive(Debug)]
pub struct CompiledSelector {
  name: globset::GlobMatcher,
  namespace: globset::GlobMatcher,
}

impl CompiledSelector {
  pub fn matches(&self, name: &str, namespace: &str) -> bool {
    self.name.is_match(name) && self.namespace.is_match(namespace)
  }
}

/// Aggregated compiled selectors for the run.
#[derive(Debug)]
pub struct CompiledChecks {
  pub all: Vec<CompiledSelector>,
  pub per_code: HashMap<Code, Vec<CompiledSelector>>,
  pub k8s002_overrides: Vec<(CompiledSelector, i32)>,
}

/// Configuration for the K8S002 minimum-replicas check.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct K8s002Config {
  /// Global minimum replica threshold (default 2).
  #[serde(default = "default_min_replicas")]
  pub min_replicas: i32,

  /// Resources to ignore entirely (no finding emitted).
  #[serde(default)]
  pub ignore: Vec<ResourceSelector>,

  /// Per-resource threshold overrides.
  #[serde(default)]
  pub overrides: Vec<ReplicaOverride>,
}

fn default_min_replicas() -> i32 {
  2
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

/// Identifies a Kubernetes resource by name + namespace.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceSelector {
  pub name: String,
  pub namespace: String,
}

impl ResourceSelector {
  pub fn compile(&self) -> Result<CompiledSelector> {
    let name = globset::Glob::new(&self.name)
      .with_context(|| format!("Invalid name glob: {}", self.name))?
      .compile_matcher();
    let namespace = globset::Glob::new(&self.namespace)
      .with_context(|| format!("Invalid namespace glob: {}", self.namespace))?
      .compile_matcher();
    Ok(CompiledSelector { name, namespace })
  }
}

impl ChecksConfig {
  pub fn compiled(&self) -> Result<&CompiledChecks> {
    if let Some(c) = self.compiled.get() {
      return Ok(c);
    }
    let built = self.build_compiled()?;
    Ok(self.compiled.get_or_init(|| built))
  }

  fn build_compiled(&self) -> Result<CompiledChecks> {
    let all = self
      .all
      .ignore
      .iter()
      .map(|s| s.compile())
      .collect::<Result<Vec<_>>>()?;

    let mut per_code = HashMap::new();
    for (code, selectors) in [
      (Code::K8S002, &self.k8s002.ignore),
      (Code::K8S003, &self.k8s003.ignore),
      (Code::K8S004, &self.k8s004.ignore),
      (Code::K8S005, &self.k8s005.ignore),
      (Code::K8S006, &self.k8s006.ignore),
      (Code::K8S007, &self.k8s007.ignore),
      (Code::K8S008, &self.k8s008.ignore),
      (Code::K8S013, &self.k8s013.ignore),
    ] {
      let compiled = selectors.iter().map(|s| s.compile()).collect::<Result<Vec<_>>>()?;
      if !compiled.is_empty() {
        per_code.insert(code, compiled);
      }
    }

    let k8s002_overrides = self
      .k8s002
      .overrides
      .iter()
      .map(|o| {
        let sel = ResourceSelector {
          name: o.name.clone(),
          namespace: o.namespace.clone(),
        };
        Ok((sel.compile()?, o.min_replicas))
      })
      .collect::<Result<Vec<_>>>()?;

    Ok(CompiledChecks {
      all,
      per_code,
      k8s002_overrides,
    })
  }
}

/// Per-resource override for the minimum replica threshold.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplicaOverride {
  pub name: String,
  pub namespace: String,
  pub min_replicas: i32,
}

impl K8s002Config {
  /// Returns the effective minimum replica threshold for a given resource.
  ///
  /// - `None` if the resource is in the ignore list (no finding should be emitted).
  /// - The override threshold if a matching override exists.
  /// - The global `min_replicas` default otherwise.
  ///
  /// Ignore takes precedence over overrides.
  pub fn effective_min_replicas(&self, name: &str, namespace: &str) -> Option<i32> {
    // Check ignore list first
    if self.ignore.iter().any(|s| s.name == name && s.namespace == namespace) {
      return None;
    }

    // Check overrides
    if let Some(ovr) = self
      .overrides
      .iter()
      .find(|o| o.name == name && o.namespace == namespace)
    {
      return Some(ovr.min_replicas);
    }

    Some(self.min_replicas)
  }
}

/// Configuration for the K8S004 PodDisruptionBudget check.
///
/// The `ignore` list uses workload names (Deployment, StatefulSet, etc.),
/// not PDB names.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct K8s004Config {
  /// Resources to ignore entirely (no finding emitted).
  #[serde(default)]
  pub ignore: Vec<ResourceSelector>,
}

impl K8s004Config {
  /// Returns true if the resource should be checked (not ignored).
  pub fn should_check(&self, name: &str, namespace: &str) -> bool {
    !self.ignore.iter().any(|s| s.name == name && s.namespace == namespace)
  }
}

impl Config {
  /// Validate configuration values after deserialization.
  fn validate(&self) -> Result<()> {
    if self.checks.k8s002.min_replicas < 1 {
      anyhow::bail!(
        "K8S002.min_replicas must be >= 1, got {}",
        self.checks.k8s002.min_replicas
      );
    }
    for ovr in &self.checks.k8s002.overrides {
      if ovr.min_replicas < 1 {
        anyhow::bail!(
          "K8S002.overrides[{}/{}].min_replicas must be >= 1, got {}",
          ovr.namespace,
          ovr.name,
          ovr.min_replicas
        );
      }
    }
    self.checks.compiled()?;
    Ok(())
  }
}

const DEFAULT_CONFIG_FILE: &str = ".eksup.yaml";

/// Load configuration from an explicit path, the default `.eksup.yaml` in the
/// current working directory, or fall back to `Config::default()`.
pub fn load(path: Option<&str>) -> Result<Config> {
  load_from(path, std::env::current_dir().ok().as_deref())
}

fn load_from(path: Option<&str>, base_dir: Option<&std::path::Path>) -> Result<Config> {
  if let Some(p) = path {
    let contents = std::fs::read_to_string(p).with_context(|| format!("Failed to read config file: {p}"))?;
    let config: Config =
      serde_yaml::from_str(&contents).with_context(|| format!("Failed to parse config file: {p}"))?;
    config.validate()?;
    return Ok(config);
  }

  // Try default path in base directory
  if let Some(dir) = base_dir {
    let default_path = dir.join(DEFAULT_CONFIG_FILE);
    if default_path.exists() {
      let contents = std::fs::read_to_string(&default_path)
        .with_context(|| format!("Failed to read config file: {}", default_path.display()))?;
      let config: Config = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse config file: {}", default_path.display()))?;
      config.validate()?;
      return Ok(config);
    }
  }

  Ok(Config::default())
}

#[cfg(test)]
mod tests {
  use std::io::Write;

  use super::*;

  // ── Default values ──────────────────────────────────────────────────

  #[test]
  fn config_default() {
    let cfg = Config::default();
    assert_eq!(cfg.checks.k8s002.min_replicas, 2);
    assert!(cfg.checks.k8s002.ignore.is_empty());
    assert!(cfg.checks.k8s002.overrides.is_empty());
  }

  #[test]
  fn k8s002_config_default() {
    let cfg = K8s002Config::default();
    assert_eq!(cfg.min_replicas, 2);
  }

  // ── effective_min_replicas ──────────────────────────────────────────

  #[test]
  fn effective_min_replicas_global_default() {
    let cfg = K8s002Config::default();
    assert_eq!(cfg.effective_min_replicas("my-app", "default"), Some(2));
  }

  #[test]
  fn effective_min_replicas_custom_global() {
    let cfg = K8s002Config {
      min_replicas: 5,
      ..Default::default()
    };
    assert_eq!(cfg.effective_min_replicas("my-app", "default"), Some(5));
  }

  #[test]
  fn effective_min_replicas_ignored() {
    let cfg = K8s002Config {
      ignore: vec![ResourceSelector {
        name: "coredns".to_string(),
        namespace: "kube-system".to_string(),
      }],
      ..Default::default()
    };
    assert_eq!(cfg.effective_min_replicas("coredns", "kube-system"), None);
  }

  #[test]
  fn effective_min_replicas_override() {
    let cfg = K8s002Config {
      overrides: vec![ReplicaOverride {
        name: "special-app".to_string(),
        namespace: "prod".to_string(),
        min_replicas: 10,
      }],
      ..Default::default()
    };
    assert_eq!(cfg.effective_min_replicas("special-app", "prod"), Some(10));
  }

  #[test]
  fn effective_min_replicas_ignore_takes_precedence_over_override() {
    let cfg = K8s002Config {
      ignore: vec![ResourceSelector {
        name: "special-app".to_string(),
        namespace: "prod".to_string(),
      }],
      overrides: vec![ReplicaOverride {
        name: "special-app".to_string(),
        namespace: "prod".to_string(),
        min_replicas: 10,
      }],
      ..Default::default()
    };
    assert_eq!(cfg.effective_min_replicas("special-app", "prod"), None);
  }

  #[test]
  fn effective_min_replicas_no_match_falls_through_to_global() {
    let cfg = K8s002Config {
      min_replicas: 3,
      ignore: vec![ResourceSelector {
        name: "other".to_string(),
        namespace: "other-ns".to_string(),
      }],
      overrides: vec![ReplicaOverride {
        name: "other".to_string(),
        namespace: "other-ns".to_string(),
        min_replicas: 99,
      }],
    };
    assert_eq!(cfg.effective_min_replicas("my-app", "default"), Some(3));
  }

  // ── K8s004Config::should_check ─────────────────────────────────────

  #[test]
  fn k8s004_should_check_default() {
    let cfg = K8s004Config::default();
    assert!(cfg.should_check("my-app", "default"));
  }

  #[test]
  fn k8s004_should_check_ignored() {
    let cfg = K8s004Config {
      ignore: vec![ResourceSelector {
        name: "coredns".to_string(),
        namespace: "kube-system".to_string(),
      }],
    };
    assert!(!cfg.should_check("coredns", "kube-system"));
  }

  #[test]
  fn k8s004_should_check_not_ignored() {
    let cfg = K8s004Config {
      ignore: vec![ResourceSelector {
        name: "coredns".to_string(),
        namespace: "kube-system".to_string(),
      }],
    };
    assert!(cfg.should_check("other-app", "default"));
  }

  // ── YAML deserialization ───────────────────────────────────────────

  #[test]
  fn deserialize_empty_yaml() {
    let yaml = "{}";
    let cfg: Config = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(cfg.checks.k8s002.min_replicas, 2);
  }

  #[test]
  fn deserialize_full_yaml() {
    let yaml = r#"
checks:
  K8S002:
    min_replicas: 5
    ignore:
      - name: coredns
        namespace: kube-system
    overrides:
      - name: my-app
        namespace: prod
        min_replicas: 10
"#;
    let cfg: Config = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(cfg.checks.k8s002.min_replicas, 5);
    assert_eq!(cfg.checks.k8s002.ignore.len(), 1);
    assert_eq!(cfg.checks.k8s002.ignore[0].name, "coredns");
    assert_eq!(cfg.checks.k8s002.overrides.len(), 1);
    assert_eq!(cfg.checks.k8s002.overrides[0].min_replicas, 10);
  }

  #[test]
  fn deserialize_k8s004_yaml() {
    let yaml = r#"
checks:
  K8S004:
    ignore:
      - name: web
        namespace: default
"#;
    let cfg: Config = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(cfg.checks.k8s004.ignore.len(), 1);
    assert_eq!(cfg.checks.k8s004.ignore[0].name, "web");
    assert_eq!(cfg.checks.k8s004.ignore[0].namespace, "default");
  }

  #[test]
  fn deserialize_empty_yaml_k8s004_defaults() {
    let yaml = "{}";
    let cfg: Config = serde_yaml::from_str(yaml).unwrap();
    assert!(cfg.checks.k8s004.ignore.is_empty());
  }

  #[test]
  fn deserialize_partial_yaml() {
    let yaml = r#"
checks:
  K8S002:
    min_replicas: 4
"#;
    let cfg: Config = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(cfg.checks.k8s002.min_replicas, 4);
    assert!(cfg.checks.k8s002.ignore.is_empty());
    assert!(cfg.checks.k8s002.overrides.is_empty());
  }

  // ── load() ─────────────────────────────────────────────────────────

  #[test]
  fn load_no_path_no_default_file() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = load_from(None, Some(tmp.path())).unwrap();
    assert_eq!(cfg.checks.k8s002.min_replicas, 2);
  }

  #[test]
  fn load_explicit_path() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("my-config.yaml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "checks:\n  K8S002:\n    min_replicas: 7").unwrap();

    let cfg = load_from(Some(path.to_str().unwrap()), None).unwrap();
    assert_eq!(cfg.checks.k8s002.min_replicas, 7);
  }

  #[test]
  fn load_explicit_path_not_found() {
    let result = load_from(Some("/tmp/does-not-exist-eksup-test.yaml"), None);
    assert!(result.is_err());
  }

  #[test]
  fn load_default_file_in_base_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let default_path = tmp.path().join(".eksup.yaml");
    let mut f = std::fs::File::create(&default_path).unwrap();
    writeln!(f, "checks:\n  K8S002:\n    min_replicas: 9").unwrap();

    let cfg = load_from(None, Some(tmp.path())).unwrap();
    assert_eq!(cfg.checks.k8s002.min_replicas, 9);
  }

  #[test]
  fn load_no_base_dir_returns_default() {
    let cfg = load_from(None, None).unwrap();
    assert_eq!(cfg.checks.k8s002.min_replicas, 2);
  }

  #[test]
  fn load_invalid_yaml_content() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("bad.yaml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "checks:\n  K8S002:\n    min_replicas: not-a-number").unwrap();
    let result = load_from(Some(path.to_str().unwrap()), None);
    assert!(result.is_err(), "invalid yaml should return Err");
  }

  #[test]
  fn load_unknown_field_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("typo.yaml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "checks:\n  K8S002:\n    min_replcia: 3").unwrap();
    let result = load_from(Some(path.to_str().unwrap()), None);
    assert!(result.is_err(), "unknown field should be rejected");
  }

  // ── validate() ──────────────────────────────────────────────────────

  #[test]
  fn validate_zero_min_replicas_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("zero.yaml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "checks:\n  K8S002:\n    min_replicas: 0").unwrap();
    let result = load_from(Some(path.to_str().unwrap()), None);
    assert!(result.is_err(), "min_replicas: 0 should be rejected");
  }

  #[test]
  fn validate_negative_min_replicas_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("negative.yaml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "checks:\n  K8S002:\n    min_replicas: -1").unwrap();
    let result = load_from(Some(path.to_str().unwrap()), None);
    assert!(result.is_err(), "negative min_replicas should be rejected");
  }

  #[test]
  fn validate_override_zero_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("bad-override.yaml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(
      f,
      "checks:\n  K8S002:\n    overrides:\n      - name: x\n        namespace: ns\n        min_replicas: 0"
    )
    .unwrap();
    let result = load_from(Some(path.to_str().unwrap()), None);
    assert!(result.is_err(), "override min_replicas: 0 should be rejected");
  }

  // ── New: WorkloadCheckConfig + glob compilation ──────────────────

  #[test]
  fn workload_check_config_default_empty() {
    let cfg = WorkloadCheckConfig::default();
    assert!(cfg.ignore.is_empty());
  }

  #[test]
  fn resource_selector_compile_literal() {
    let sel = ResourceSelector {
      name: "my-app".to_string(),
      namespace: "default".to_string(),
    };
    let compiled = sel.compile().unwrap();
    assert!(compiled.matches("my-app", "default"));
    assert!(!compiled.matches("other-app", "default"));
    assert!(!compiled.matches("my-app", "other-ns"));
  }

  #[test]
  fn resource_selector_compile_glob() {
    let sel = ResourceSelector {
      name: "preview-*".to_string(),
      namespace: "*-dev".to_string(),
    };
    let compiled = sel.compile().unwrap();
    assert!(compiled.matches("preview-foo", "team-a-dev"));
    assert!(compiled.matches("preview-bar", "team-b-dev"));
    assert!(!compiled.matches("prod-foo", "team-a-dev"));
    assert!(!compiled.matches("preview-foo", "production"));
  }

  #[test]
  fn resource_selector_compile_brace_expansion() {
    let sel = ResourceSelector {
      name: "{web,api}-*".to_string(),
      namespace: "prod".to_string(),
    };
    let compiled = sel.compile().unwrap();
    assert!(compiled.matches("web-a", "prod"));
    assert!(compiled.matches("api-b", "prod"));
    assert!(!compiled.matches("worker-a", "prod"));
  }

  #[test]
  fn resource_selector_compile_invalid_glob_errors() {
    let sel = ResourceSelector {
      name: "[unclosed".to_string(),
      namespace: "default".to_string(),
    };
    let err = sel.compile().unwrap_err();
    assert!(err.to_string().contains("Invalid name glob"));
  }

  #[test]
  fn checks_config_default_includes_all_workload_codes() {
    let cfg = ChecksConfig::default();
    assert!(cfg.all.ignore.is_empty());
    assert!(cfg.k8s003.ignore.is_empty());
    assert!(cfg.k8s005.ignore.is_empty());
    assert!(cfg.k8s006.ignore.is_empty());
    assert!(cfg.k8s007.ignore.is_empty());
    assert!(cfg.k8s008.ignore.is_empty());
    assert!(cfg.k8s013.ignore.is_empty());
  }

  #[test]
  fn deserialize_all_block() {
    let yaml = r#"
checks:
  all:
    ignore:
      - name: "*"
        namespace: "*-dev*"
"#;
    let cfg: Config = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(cfg.checks.all.ignore.len(), 1);
    assert_eq!(cfg.checks.all.ignore[0].name, "*");
    assert_eq!(cfg.checks.all.ignore[0].namespace, "*-dev*");
  }

  #[test]
  fn deserialize_k8s003_through_k8s013() {
    let yaml = r#"
checks:
  K8S003: { ignore: [{ name: "a", namespace: "x" }] }
  K8S005: { ignore: [{ name: "b", namespace: "y" }] }
  K8S006: { ignore: [{ name: "c", namespace: "z" }] }
  K8S007: { ignore: [{ name: "d", namespace: "w" }] }
  K8S008: { ignore: [{ name: "e", namespace: "v" }] }
  K8S013: { ignore: [{ name: "g", namespace: "t" }] }
"#;
    let cfg: Config = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(cfg.checks.k8s003.ignore[0].name, "a");
    assert_eq!(cfg.checks.k8s005.ignore[0].name, "b");
    assert_eq!(cfg.checks.k8s006.ignore[0].name, "c");
    assert_eq!(cfg.checks.k8s007.ignore[0].name, "d");
    assert_eq!(cfg.checks.k8s008.ignore[0].name, "e");
    assert_eq!(cfg.checks.k8s013.ignore[0].name, "g");
  }

  #[test]
  fn k8s012_is_not_a_config_field() {
    // K8S012 (KubeProxyIpvsMode) is cluster-level — has no resource field.
    let yaml = r#"
checks:
  K8S012: { ignore: [{ name: "kube-proxy", namespace: "kube-system" }] }
"#;
    let result: Result<Config, _> = serde_yaml::from_str(yaml);
    assert!(
      result.is_err(),
      "K8S012 in checks: should be rejected (cluster-level check)"
    );
  }

  #[test]
  fn deserialize_unknown_check_code_rejected() {
    let yaml = r#"
checks:
  AWS001:
    ignore:
      - name: foo
        namespace: bar
"#;
    let result: Result<Config, _> = serde_yaml::from_str(yaml);
    assert!(
      result.is_err(),
      "cluster-level code AWS001 in checks: should be rejected"
    );
  }

  #[test]
  fn validate_compiles_globs_lazily_and_errors_on_invalid() {
    let yaml = r#"
checks:
  K8S003:
    ignore:
      - name: "[unclosed"
        namespace: "default"
"#;
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("bad-glob.yaml");
    std::fs::write(&path, yaml).unwrap();
    let result = load_from(Some(path.to_str().unwrap()), None);
    assert!(result.is_err(), "invalid glob in K8S003.ignore should error at load");
  }

  #[test]
  fn compiled_checks_caches_all_codes() {
    let yaml = r#"
checks:
  all:
    ignore: [{ name: "*-tmp", namespace: "default" }]
  K8S002:
    ignore: [{ name: "k8s002-app", namespace: "default" }]
  K8S013:
    ignore: [{ name: "ingress-*", namespace: "ingress" }]
"#;
    let cfg: Config = serde_yaml::from_str(yaml).unwrap();
    cfg.validate().unwrap();
    let compiled = cfg.checks.compiled().unwrap();
    assert_eq!(compiled.all.len(), 1);
    assert!(compiled.per_code.contains_key(&Code::K8S002));
    assert!(compiled.per_code.contains_key(&Code::K8S013));
    assert!(!compiled.per_code.contains_key(&Code::K8S003));
  }
}
