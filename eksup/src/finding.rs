use anyhow::Result;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::version;

#[derive(Clone, Debug, Serialize, Deserialize, Tabled)]
#[tabled(rename_all = "UpperCase")]
pub struct Finding {
  #[tabled(rename = "CHECK")]
  pub code: Code,
  #[tabled(rename = " ")]
  pub symbol: String,
  #[tabled(skip)]
  pub remediation: Remediation,
}

impl Finding {
  pub fn new(code: Code, remediation: Remediation) -> Self {
    Self {
      code,
      symbol: remediation.symbol(),
      remediation,
    }
  }
}

/// Determines whether remediation is required or recommended
///
/// This allows for filtering of findings shown to user
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Remediation {
  /// A finding that requires remediation prior to upgrading to be able to perform the upgrade
  /// and avoid downtime or disruption
  Required,
  /// A finding that users are encouraged to evaluate the recommendation and determine if it
  /// is applicable and whether or not to act upon that recommendation.
  /// Not remediating the finding does not prevent the upgrade from occurring.
  Recommended,
}

impl Remediation {
  pub(crate) fn symbol(&self) -> String {
    match &self {
      Remediation::Required => "❌".to_string(),
      Remediation::Recommended => "⚠️".to_string(),
    }
  }

  pub(crate) fn is_recommended(&self) -> bool {
    matches!(self, Remediation::Recommended)
  }
}

impl std::fmt::Display for Remediation {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match *self {
      Remediation::Required => write!(f, "Required"),
      Remediation::Recommended => write!(f, "Recommended"),
    }
  }
}

pub trait Findings {
  fn to_markdown_table(&self, leading_whitespace: &str) -> Result<String>;
  fn to_stdout_table(&self) -> Result<String>;
}

macro_rules! impl_findings {
  ($type:ty, $empty_msg:expr) => {
    impl Findings for Vec<$type> {
      fn to_markdown_table(&self, leading_whitespace: &str) -> ::anyhow::Result<String> {
        if self.is_empty() {
          return Ok(format!("{leading_whitespace}{}", $empty_msg));
        }

        let mut table = ::tabled::Table::new(self);
        table
          .with(::tabled::settings::Remove::column(::tabled::settings::location::ByColumnName::new("CHECK")))
          .with(::tabled::settings::Margin::new(1, 0, 0, 0).fill('\t', 'x', 'x', 'x'))
          .with(::tabled::settings::Style::markdown());

        Ok(format!("{table}\n"))
      }

      fn to_stdout_table(&self) -> ::anyhow::Result<String> {
        if self.is_empty() {
          return Ok(String::new());
        }

        let mut table = ::tabled::Table::new(self);
        table.with(::tabled::settings::Style::sharp());

        Ok(format!("{table}\n"))
      }
    }
  };
}

pub(crate) use impl_findings;

/// Codes that represent the finding variants
///
/// This is useful for a few reasons:
/// 1. It would allow users to add codes to a 'ignore list' in the future, to ignore any reported findings of that code
///    type (another level of granularity of what data is is most relevant to them)
/// 2. It provides a "marker" that can be used to link to documentation for the finding, keeping the direct output
///    concise while still providing the means for a full explanation and reasoning behind the finding in one location
/// 3. It provides a strongly typed link between code and finding data allowing the code to uniquely represent a finding
///    even if the finding data is generic (i.e. - as is the case in reporting available IPs as subnet findings, the
///    data shape is generic by the finding is unique to different scenarios)
macro_rules! define_codes {
  ($( $variant:ident => {
    desc: $desc:expr,
    from: $from:expr,
    until: $until:expr $(,)?
  }),* $(,)?) => {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum Code { $($variant,)* }

    impl std::fmt::Display for Code {
      fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self { $(Code::$variant => write!(f, stringify!($variant)),)* }
      }
    }

    #[allow(dead_code)]
    impl Code {
      pub(crate) fn description(&self) -> &'static str {
        match self { $(Code::$variant => $desc,)* }
      }

      /// Minimum target minor version where this check becomes relevant (inclusive).
      /// `None` means always relevant.
      pub(crate) fn applicable_from(&self) -> Option<i32> {
        match self { $(Code::$variant => $from,)* }
      }

      /// Maximum target minor version where this check is relevant (inclusive).
      /// `None` means still relevant for all future versions.
      pub(crate) fn applicable_until(&self) -> Option<i32> {
        match self { $(Code::$variant => $until,)* }
      }
    }
  };
}

define_codes! {
  AWS001 => { desc: "Insufficient available subnet IPs for nodes",                   from: None,     until: None },
  AWS002 => { desc: "Insufficient available subnet IPs for pods (custom networking)", from: None,     until: None },
  AWS003 => { desc: "Insufficient EC2 service limits",                               from: None,     until: None },
  AWS004 => { desc: "Insufficient EBS GP2 service limits",                           from: None,     until: None },
  AWS005 => { desc: "Insufficient EBS GP3 service limits",                           from: None,     until: None },
  EKS001 => { desc: "Insufficient available subnet IPs for control plane ENIs",      from: None,     until: None },
  EKS002 => { desc: "Health issue(s) reported by the EKS control plane",             from: None,     until: None },
  EKS003 => { desc: "Health issue(s) reported by the EKS managed node group",        from: None,     until: None },
  EKS004 => { desc: "Health issue(s) reported by the EKS addon",                     from: None,     until: None },
  EKS005 => { desc: "EKS addon incompatible with targeted Kubernetes version",       from: None,     until: None },
  EKS006 => { desc: "EKS managed node group has pending launch template update(s)",  from: None,     until: None },
  EKS007 => { desc: "Self-managed node group has pending launch template update(s)", from: None,     until: None },
  EKS008 => { desc: "AL2 AMI deprecation (deprecated in 1.32, removed in 1.33+)",   from: Some(32), until: None },
  K8S001 => { desc: "Kubernetes version skew between control plane and node",        from: None,     until: None },
  K8S002 => { desc: "Insufficient number of .spec.replicas",                         from: None,     until: None },
  K8S003 => { desc: "Insufficient .spec.minReadySeconds",                            from: None,     until: None },
  K8S004 => { desc: "Missing PodDisruptionBudget",                                   from: None,     until: None },
  K8S005 => { desc: "Pod distribution settings put availability at risk",            from: None,     until: None },
  K8S006 => { desc: "Missing readinessProbe on containers",                          from: None,     until: None },
  K8S007 => { desc: "TerminationGracePeriodSeconds is set to zero",                  from: None,     until: None },
  K8S008 => { desc: "Mounts docker.sock or dockershim.sock",                         from: None,     until: None },
  K8S009 => { desc: "Pod security policies present (removed in 1.25)",               from: None,     until: Some(24) },
  K8S010 => { desc: "EBS CSI driver not installed",                                  from: None,     until: None },
  K8S011 => { desc: "kube-proxy version skew with kubelet",                          from: None,     until: None },
  K8S012 => { desc: "kube-proxy IPVS mode deprecated (1.35+, removed 1.36)",        from: Some(35), until: None },
  K8S013 => { desc: "Ingress NGINX controller retirement (1.35+)",                   from: Some(35), until: None },
}

#[allow(dead_code)]
impl Code {
  /// Whether the check applies for a given target minor version.
  pub fn is_applicable(&self, target_minor: i32) -> bool {
    if let Some(from) = self.applicable_from()
      && target_minor < from
    {
      return false;
    }

    if let Some(until) = self.applicable_until()
      && target_minor > until
    {
      return false;
    }

    true
  }

  /// Whether the check is retired (`applicable_until` falls below `MINIMUM`).
  pub fn is_retired(&self) -> bool {
    match self.applicable_until() {
      Some(until) => until < version::MINIMUM,
      None => false,
    }
  }

  pub(crate) fn url(&self) -> String {
    format!(
      "https://clowdhaus.github.io/eksup/info/checks/#{}",
      self.to_string().to_lowercase()
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn eks008_applicable_from_1_32() {
    assert!(!Code::EKS008.is_applicable(31));
    assert!(Code::EKS008.is_applicable(32));
    assert!(Code::EKS008.is_applicable(35));
  }

  #[test]
  fn k8s009_applicable_until_1_24() {
    assert!(Code::K8S009.is_applicable(24));
    assert!(!Code::K8S009.is_applicable(25));
    assert!(!Code::K8S009.is_applicable(30));
  }

  #[test]
  fn k8s009_is_retired() {
    assert!(Code::K8S009.is_retired());
  }

  #[test]
  fn always_relevant_codes_apply_to_any_version() {
    let always = [
      Code::AWS001, Code::AWS002, Code::EKS001, Code::K8S001,
      Code::K8S002, Code::K8S008, Code::K8S011,
    ];
    for code in &always {
      assert!(code.is_applicable(30), "{code} should be applicable at 1.30");
      assert!(code.is_applicable(35), "{code} should be applicable at 1.35");
      assert!(!code.is_retired(), "{code} should not be retired");
    }
  }

  #[test]
  fn k8s012_k8s013_applicable_from_1_35() {
    assert!(!Code::K8S012.is_applicable(34));
    assert!(Code::K8S012.is_applicable(35));
    assert!(!Code::K8S013.is_applicable(34));
    assert!(Code::K8S013.is_applicable(35));
  }

  #[test]
  fn code_display() {
    assert_eq!(Code::AWS001.to_string(), "AWS001");
    assert_eq!(Code::EKS008.to_string(), "EKS008");
    assert_eq!(Code::K8S013.to_string(), "K8S013");
  }

  #[test]
  fn code_description() {
    assert_eq!(Code::AWS001.description(), "Insufficient available subnet IPs for nodes");
    assert_eq!(Code::EKS008.description(), "AL2 AMI deprecation (deprecated in 1.32, removed in 1.33+)");
    assert!(Code::K8S009.description().contains("Pod security policies"));
  }

  #[test]
  fn code_url() {
    assert_eq!(Code::AWS001.url(), "https://clowdhaus.github.io/eksup/info/checks/#aws001");
    assert_eq!(Code::K8S013.url(), "https://clowdhaus.github.io/eksup/info/checks/#k8s013");
  }

  #[test]
  fn remediation_display() {
    assert_eq!(Remediation::Required.to_string(), "Required");
    assert_eq!(Remediation::Recommended.to_string(), "Recommended");
  }

  #[test]
  fn remediation_symbol() {
    assert_eq!(Remediation::Required.symbol(), "❌");
    assert_eq!(Remediation::Recommended.symbol(), "⚠️");
  }
}
