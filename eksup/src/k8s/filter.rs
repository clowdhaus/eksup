//! Generic ignore-filter for workload-level findings.

use crate::config::CompiledChecks;
use crate::finding::WorkloadFinding;

/// Split findings into (kept, suppressed) based on ignore rules in `compiled`.
///
/// A finding is suppressed when its `(name, namespace)` matches any selector
/// in `compiled.all` or in `compiled.per_code[finding.code()]`.
pub fn apply_ignores<T: WorkloadFinding>(
  findings: Vec<T>,
  compiled: &CompiledChecks,
) -> (Vec<T>, Vec<T>) {
  let mut kept = Vec::with_capacity(findings.len());
  let mut suppressed = Vec::new();

  for f in findings {
    let (name, ns) = f.resource();
    let matched_all = compiled.all.iter().any(|s| s.matches(name, ns));
    let matched_code = compiled
      .per_code
      .get(&f.code())
      .map(|sels| sels.iter().any(|s| s.matches(name, ns)))
      .unwrap_or(false);

    if matched_all || matched_code {
      suppressed.push(f);
    } else {
      kept.push(f);
    }
  }
  (kept, suppressed)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::config::ChecksConfig;
  use crate::finding::Code;

  #[derive(Debug, PartialEq)]
  struct TestFinding {
    name: String,
    namespace: String,
    code: Code,
  }

  impl WorkloadFinding for TestFinding {
    fn resource(&self) -> (&str, &str) {
      (&self.name, &self.namespace)
    }
    fn code(&self) -> Code {
      self.code.clone()
    }
  }

  fn finding(name: &str, ns: &str, code: Code) -> TestFinding {
    TestFinding {
      name: name.to_string(),
      namespace: ns.to_string(),
      code,
    }
  }

  fn cfg(
    all: Vec<(&str, &str)>,
    k8s002: Vec<(&str, &str)>,
    k8s003: Vec<(&str, &str)>,
  ) -> ChecksConfig {
    let render = |v: Vec<(&str, &str)>| -> String {
      v.into_iter()
        .map(|(n, ns)| format!("      - name: \"{n}\"\n        namespace: \"{ns}\"\n"))
        .collect::<String>()
    };
    let mut yaml = String::from("all:\n  ignore:\n");
    yaml.push_str(&render(all));
    yaml.push_str("K8S002:\n  min_replicas: 2\n  ignore:\n");
    yaml.push_str(&render(k8s002));
    yaml.push_str("K8S003:\n  ignore:\n");
    yaml.push_str(&render(k8s003));
    serde_yaml::from_str(&yaml).expect("test cfg yaml should parse")
  }

  #[test]
  fn empty_config_keeps_everything() {
    let findings = vec![
      finding("a", "ns", Code::K8S002),
      finding("b", "ns", Code::K8S003),
    ];
    let config = ChecksConfig::default();
    let compiled = config.compiled().unwrap();
    let (kept, suppressed) = apply_ignores(findings, compiled);
    assert_eq!(kept.len(), 2);
    assert_eq!(suppressed.len(), 0);
  }

  #[test]
  fn all_ignore_suppresses_across_codes() {
    let findings = vec![
      finding("dev-a", "team-dev", Code::K8S002),
      finding("dev-b", "team-dev", Code::K8S003),
      finding("prod-a", "prod", Code::K8S002),
    ];
    let config = cfg(vec![("*", "*-dev")], vec![], vec![]);
    let compiled = config.compiled().unwrap();
    let (kept, suppressed) = apply_ignores(findings, compiled);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].name, "prod-a");
    assert_eq!(suppressed.len(), 2);
  }

  #[test]
  fn per_check_ignore_scoped_to_that_code() {
    let findings = vec![
      finding("app", "default", Code::K8S002),
      finding("app", "default", Code::K8S003),
    ];
    let config = cfg(vec![], vec![("app", "default")], vec![]);
    let compiled = config.compiled().unwrap();
    let (kept, suppressed) = apply_ignores(findings, compiled);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].code, Code::K8S003);
    assert_eq!(suppressed.len(), 1);
    assert_eq!(suppressed[0].code, Code::K8S002);
  }

  #[test]
  fn both_all_and_per_check_match_suppresses_once() {
    let findings = vec![finding("app", "default", Code::K8S002)];
    let config = cfg(vec![("*", "default")], vec![("app", "*")], vec![]);
    let compiled = config.compiled().unwrap();
    let (kept, suppressed) = apply_ignores(findings, compiled);
    assert_eq!(kept.len(), 0);
    assert_eq!(suppressed.len(), 1);
  }

  #[test]
  fn glob_match_preserves_order() {
    let findings = vec![
      finding("z-app", "default", Code::K8S002),
      finding("a-app", "default", Code::K8S002),
      finding("m-app", "default", Code::K8S002),
    ];
    let config = cfg(vec![], vec![("*-app", "default")], vec![]);
    let compiled = config.compiled().unwrap();
    let (kept, suppressed) = apply_ignores(findings, compiled);
    assert_eq!(kept.len(), 0);
    assert_eq!(suppressed.len(), 3);
    assert_eq!(suppressed[0].name, "z-app");
    assert_eq!(suppressed[1].name, "a-app");
    assert_eq!(suppressed[2].name, "m-app");
  }

  #[test]
  fn no_match_keeps_finding() {
    let findings = vec![finding("real-app", "default", Code::K8S002)];
    let config = cfg(vec![("test-*", "*")], vec![("staging-*", "*")], vec![]);
    let compiled = config.compiled().unwrap();
    let (kept, suppressed) = apply_ignores(findings, compiled);
    assert_eq!(kept.len(), 1);
    assert_eq!(suppressed.len(), 0);
  }

  #[test]
  fn filter_is_idempotent() {
    let findings = vec![finding("app", "default", Code::K8S002)];
    let config = cfg(vec![], vec![("app", "default")], vec![]);
    let compiled = config.compiled().unwrap();
    let (kept1, suppressed1) = apply_ignores(findings, compiled);
    let (kept2, suppressed2) = apply_ignores(kept1, compiled);
    assert_eq!(kept2.len(), 0);
    assert_eq!(suppressed1.len(), 1);
    assert_eq!(suppressed2.len(), 0);
  }

  #[test]
  fn brace_pattern_in_namespace_matches() {
    let findings = vec![
      finding("app", "prod", Code::K8S002),
      finding("app", "staging", Code::K8S002),
      finding("app", "dev", Code::K8S002),
    ];
    let config = cfg(vec![], vec![("*", "{prod,staging}")], vec![]);
    let compiled = config.compiled().unwrap();
    let (kept, suppressed) = apply_ignores(findings, compiled);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].namespace, "dev");
    assert_eq!(suppressed.len(), 2);
  }
}
