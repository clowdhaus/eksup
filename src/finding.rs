use serde::{Deserialize, Serialize};

use crate::version;

/// Determines whether remediation is required or recommended
///
/// This allows for filtering of findings shown to user
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Remediation {
  /// A finding that requires remediation prior to upgrading to avoid downtime or disruption
  Required,
  /// A finding that users are recommended to remediate prior to upgrade, but failure
  /// to do so does not pose a risk to downtime or disruption during the upgrade
  Recommended,
}

impl Remediation {
  pub(crate) fn symbol(&self) -> &'static str {
    match &self {
      Remediation::Required => "‼",
      Remediation::Recommended => "ℹ️",
    }
  }
}

pub(crate) trait Findings {
  fn to_markdown_table(&self) -> Option<String>;
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
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Code {
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

  /// Incorrect update strategy is used
  K8S004,

  /// Missing `podDisruptionBudgets`
  K8S005,

  /// Pod distribution settings put availability at risk
  K8S006,

  /// `pod.spec.containers[*].readinessProbe` not set
  K8S007,

  /// `pod.spec.TerminationGracePeriodSeconds` is set to zero
  K8S008,
}
