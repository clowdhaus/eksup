use anyhow::Result;
use kube::Client as K8sClient;
use serde::{Deserialize, Serialize};

use crate::k8s::{
  checks::{self, K8sFindings},
  resources,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct KubernetesFindings {
  pub min_replicas: Vec<checks::MinReplicas>,
  pub min_ready_seconds: Vec<checks::MinReadySeconds>,
  pub readiness_probe: Vec<checks::Probe>,
  pub pod_topology_distribution: Vec<checks::PodTopologyDistribution>,
  pub termination_grace_period: Vec<checks::TerminationGracePeriod>,
}

pub async fn get_kubernetes_findings(k8s_client: &K8sClient) -> Result<KubernetesFindings> {
  let resources = resources::get_resources(k8s_client).await?;

  let min_replicas: Vec<checks::MinReplicas> = resources.iter().filter_map(|s| s.min_replicas()).collect();
  let min_ready_seconds: Vec<checks::MinReadySeconds> =
    resources.iter().filter_map(|s| s.min_ready_seconds()).collect();
  let readiness_probe: Vec<checks::Probe> = resources.iter().filter_map(|s| s.readiness_probe()).collect();
  let pod_topology_distribution: Vec<checks::PodTopologyDistribution> =
    resources.iter().filter_map(|s| s.pod_topology_distribution()).collect();
  let termination_grace_period: Vec<checks::TerminationGracePeriod> =
    resources.iter().filter_map(|s| s.termination_grace_period()).collect();

  Ok(KubernetesFindings {
    min_replicas,
    min_ready_seconds,
    readiness_probe,
    pod_topology_distribution,
    termination_grace_period,
  })
}
