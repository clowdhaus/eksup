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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Code {
  /// AWS finding codes not specific to EKS
  ///
  /// Insufficient available subnet IPs for nodes
  AWS001,

  /// Insufficient available subnet IPs for pods (custom networking only)
  AWS002,

  /// Insufficient EC2 service limits
  AWS003,

  /// Insufficient EBS GP2 service limits
  AWS004,

  /// Insufficient EBS GP3 service limits
  AWS005,

  /// EKS specific finding codes
  ///
  /// Insufficient available subnet IPs (5 min) for control plane ENIs
  EKS001,

  /// Health issue(s) reported by the EKS control plane
  EKS002,

  /// Health issue(s) reported by the EKS managed node group
  EKS003,

  /// Health issue(s) reported by the EKS addon
  EKS004,

  /// EKS addon is incompatible with the targeted Kubernetes version
  EKS005,

  /// EKS managed node group autoscaling group has pending update(s)
  EKS006,

  /// Self-managed node group autoscaling group has pending update(s)
  EKS007,

  /// AL2 AMI deprecation (deprecated in 1.32, removed in 1.33+)
  EKS008,

  /// Kubernetes finding codes not specific to EKS
  ///
  /// Kubernetes version skew detected between control plane and node
  K8S001,

  /// Insufficient number of `.spec.replicas`
  K8S002,

  /// Insufficient number of `.spec.minReadySeconds`
  K8S003,

  /// Missing `podDisruptionBudgets`
  K8S004,

  /// Pod distribution settings put availability at risk
  K8S005,

  /// `pod.spec.containers[*].readinessProbe` not set
  K8S006,

  /// `pod.spec.TerminationGracePeriodSeconds` is set to zero
  K8S007,

  /// Mounts `docker.sock` or `dockershim.sock`
  K8S008,

  /// Pod security policies present
  K8S009,

  /// EBS CSI driver not installed (v1.23+)
  K8S010,

  /// Kubernetes version skew detected between kube-proxy and kubelet
  K8S011,

  /// kube-proxy IPVS mode deprecated (deprecated in 1.35, removed in 1.36)
  K8S012,

  /// Ingress NGINX controller retirement (recommended for 1.35+)
  K8S013,
}

#[allow(dead_code)]
impl Code {
  /// Minimum target minor version where this check becomes relevant (inclusive).
  /// `None` means always relevant.
  pub fn applicable_from(&self) -> Option<i32> {
    match self {
      Code::EKS008 => Some(32),
      Code::K8S012 | Code::K8S013 => Some(35),
      _ => None,
    }
  }

  /// Maximum target minor version where this check is relevant (inclusive).
  /// `None` means still relevant for all future versions.
  pub fn applicable_until(&self) -> Option<i32> {
    match self {
      Code::K8S009 => Some(24),
      _ => None,
    }
  }

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
}

impl std::fmt::Display for Code {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match *self {
      Code::AWS001 => write!(f, "AWS001"),
      Code::AWS002 => write!(f, "AWS002"),
      Code::AWS003 => write!(f, "AWS003"),
      Code::AWS004 => write!(f, "AWS004"),
      Code::AWS005 => write!(f, "AWS005"),
      Code::EKS001 => write!(f, "EKS001"),
      Code::EKS002 => write!(f, "EKS002"),
      Code::EKS003 => write!(f, "EKS003"),
      Code::EKS004 => write!(f, "EKS004"),
      Code::EKS005 => write!(f, "EKS005"),
      Code::EKS006 => write!(f, "EKS006"),
      Code::EKS007 => write!(f, "EKS007"),
      Code::EKS008 => write!(f, "EKS008"),
      Code::K8S001 => write!(f, "K8S001"),
      Code::K8S002 => write!(f, "K8S002"),
      Code::K8S003 => write!(f, "K8S003"),
      Code::K8S004 => write!(f, "K8S004"),
      Code::K8S005 => write!(f, "K8S005"),
      Code::K8S006 => write!(f, "K8S006"),
      Code::K8S007 => write!(f, "K8S007"),
      Code::K8S008 => write!(f, "K8S008"),
      Code::K8S009 => write!(f, "K8S009"),
      Code::K8S010 => write!(f, "K8S010"),
      Code::K8S011 => write!(f, "K8S011"),
      Code::K8S012 => write!(f, "K8S012"),
      Code::K8S013 => write!(f, "K8S013"),
    }
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
}
