use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Top-level configuration loaded from `.eksup.yaml` or an explicit path.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Config {
  #[serde(default)]
  pub checks: ChecksConfig,
}

/// Per-check configuration knobs.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChecksConfig {
  #[serde(default, rename = "K8S002")]
  pub k8s002: K8s002Config,

  #[serde(default, rename = "K8S004")]
  pub k8s004: K8s004Config,
}

/// Configuration for the K8S002 minimum-replicas check.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
pub struct ResourceSelector {
  pub name: String,
  pub namespace: String,
}

/// Per-resource override for the minimum replica threshold.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    if let Some(ovr) = self.overrides.iter().find(|o| o.name == name && o.namespace == namespace) {
      return Some(ovr.min_replicas);
    }

    Some(self.min_replicas)
  }
}

/// Configuration for the K8S004 PodDisruptionBudget check.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
      return Ok(config);
    }
  }

  Ok(Config::default())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Write;

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
      ..Default::default()
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
}
