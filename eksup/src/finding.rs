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

/// TODO - something is required to identify what Kubernetes resource findings are applicable
/// TODO - to specific version. For example, if a user is already on version 1.23, then they should
/// TODO - not be shown findings that affect version <= 1.22
pub(crate) trait Deprecation {
  /// Returns the Kubernetes version the check was deprecated in
  fn deprecated_in(&self) -> Option<version::KubernetesVersion>;
  /// Returns the Kubernetes version the check will be removed in
  fn removed_in(&self) -> Option<version::KubernetesVersion>;
}

/// Codes that represent the finding variants
///
/// This is useful for a few reasons:
/// 1. It would allow users to add codes to a 'ignore list' in the future, to ignore any
/// reported findings of that code type (another level of granularity of what data is
/// is most relevant to them)
/// 2. It provides a "marker" that can be used to link to documentation for the finding,
/// keeping the direct output concise while still providing the means for a full explanation
/// and reasoning behind the finding in one location
/// 3. It provides a strongly typed link between code and finding data allowing the code
/// to uniquely represent a finding even if the finding data is generic (i.e. - as is the case
/// in reporting available IPs as subnet findings, the data shape is generic by the finding
/// is unique to different scenarios)
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
      Code::K8S001 => write!(f, "K8S001"),
      Code::K8S002 => write!(f, "K8S002"),
      Code::K8S003 => write!(f, "K8S003"),
      Code::K8S004 => write!(f, "K8S004"),
      Code::K8S005 => write!(f, "K8S005"),
      Code::K8S006 => write!(f, "K8S006"),
      Code::K8S007 => write!(f, "K8S007"),
      Code::K8S008 => write!(f, "K8S008"),
    }
  }
}
