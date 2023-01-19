use serde::{Deserialize, Serialize};

use crate::{eks, k8s, version};

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Remediation {
  /// Represents a finding that requires remediation prior to upgrading
  Required,
  /// Represents a finding that is suggested as a recommendation
  Recommended,
}

pub(crate) trait Deprecation {
  /// Returns the Kubernetes version the check was deprecated in
  fn deprecated_in(&self) -> Option<version::KubernetesVersion>;
  /// Returns the Kubernetes version the check will be removed in
  fn removed_in(&self) -> Option<version::KubernetesVersion>;
}

pub(crate) type FindingResult = Result<Option<Code>, anyhow::Error>;
pub(crate) type FindingResults = Result<Vec<Code>, anyhow::Error>;

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Code {
  /// AWS finding codes not specific to EKS
  ///
  /// Insufficient available subnet IPs for nodes
  AWS001(eks::InsufficientSubnetIps),

  /// Insufficient available subnet IPs for pods (custom networking only)
  AWS002(eks::InsufficientSubnetIps),

  /// Insufficient EC2 service limits
  AWS003,

  /// Insufficient EBS GP2 service limits
  AWS004,

  /// Insufficient EBS GP3 service limits
  AWS005,

  /// EKS specific finding codes
  ///
  /// Insufficient available subnet IPs (5 min) for control plane ENIs
  EKS001(eks::InsufficientSubnetIps),

  /// Health issue(s) reported by the EKS control plane
  EKS002(eks::ClusterHealthIssue),

  /// Health issue(s) reported by the EKS managed node group
  EKS003(eks::NodegroupHealthIssue),

  /// Health issue(s) reported by the EKS addon
  EKS004(eks::AddonHealthIssue),

  /// EKS addon is incompatible with the targeted Kubernetes version
  EKS005(eks::AddonVersionCompatibility),

  /// EKS managed node group autoscaling group has pending update(s)
  EKS006(eks::ManagedNodeGroupUpdate),

  /// Self-managed node group autoscaling group has pending update(s)
  EKS007(eks::AutoscalingGroupUpdate),

  /// Kubernetes finding codes not specific to EKS
  ///
  /// Kubernetes version skew detected between control plane and node
  K8S001(k8s::NodeFinding),

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
