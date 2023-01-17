use serde::{Deserialize, Serialize};

use crate::k8s;

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Remediation {
  /// Represents a finding that requires remediation prior to upgrading
  Required,
  /// Represents a finding that is suggested as a recommendation
  Recommended,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Code {
  /// AWS finding codes not specific to EKS
  ///
  /// AWS101: Kubernetes version skew detected between control plane and node
  AWS101(k8s::NodeFinding),
  // /// EKS specific finding codes
  // ///
  // EKS101(Finding),

  // /// Kubernetes finding codes not specific to EKS
  // ///
  // K8S001(Finding),
}
