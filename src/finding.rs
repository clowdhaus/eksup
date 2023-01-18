use serde::{Deserialize, Serialize};

use crate::{analysis, k8s, version};

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Remediation {
  /// Represents a finding that requires remediation prior to upgrading
  Required,
  /// Represents a finding that is suggested as a recommendation
  Recommended,
}

pub trait Check {
  /// Returns the Kubernetes version the check was deprecated in
  fn deprecated_in(&self) -> Option<version::KubernetesVersion>;

  fn removed_in(&self) -> Option<version::KubernetesVersion>;
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Code {
  /// AWS finding codes not specific to EKS
  ///
  /// Insufficient available subnet IPs for nodes
  AWS001,

  /// Insufficient available subnet IPs for pods (custom networking only)
  AWS002(analysis::SubnetFinding),

  /// Insufficient EC2 service limits
  AWS003(analysis::SubnetFinding),

  /// Insufficient EBS GP2 service limits
  AWS004,

  /// Insufficient EBS GP3 service limits
  AWS005,

  /// EKS specific finding codes
  ///
  /// Insufficient available subnet IPs (5 min) for control plane ENIs
  EKS001(analysis::SubnetFinding),

  /// Health issue(s) reported by the EKS control plane
  EKS002,

  /// Health issue(s) reported by the EKS managed node group
  EKS003,

  /// Health issue(s) reported by the EKS addon
  EKS004,

  /// EKS addon is incompatible with the targetted Kubernetes version
  EKS005,

  /// EKS managed node group autoscaling group has pending update(s)
  EKS006,

  /// Self-managed node group autoscaling group has pending update(s)
  EKS007,

  /// Kubernetes finding codes not specific to EKS
  ///
  /// Kubernetes version skew detected between control plane and node
  K8S001(k8s::NodeFinding),

  /// Insufficient number of `.spec.replcas`
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
