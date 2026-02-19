use anyhow::Result;
use kube::Client as K8sClient;
use serde::{Deserialize, Serialize};

use crate::k8s::{
  checks::{self, K8sFindings},
  resources,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct KubernetesFindings {
  /// The skew/diff between the cluster control plane (API Server) and the nodes in the data plane (kubelet)
  /// It is recommended that these versions are aligned prior to upgrading, and changes are required when
  /// the skew policy could be violated post upgrade (i.e. if current skew is +2, the policy would be violated
  /// as soon as the control plane is upgraded, resulting in +3, and therefore changes are required before upgrade)
  pub version_skew: Vec<checks::VersionSkew>,
  pub min_replicas: Vec<checks::MinReplicas>,
  pub min_ready_seconds: Vec<checks::MinReadySeconds>,
  pub readiness_probe: Vec<checks::Probe>,
  pub pod_topology_distribution: Vec<checks::PodTopologyDistribution>,
  pub termination_grace_period: Vec<checks::TerminationGracePeriod>,
  pub docker_socket: Vec<checks::DockerSocket>,
  pub kube_proxy_version_skew: Vec<checks::KubeProxyVersionSkew>,
}

pub async fn get_kubernetes_findings(
  client: &K8sClient,
  cluster_version: &str,
  target_version: &str,
) -> Result<KubernetesFindings> {
  let resources = resources::get_resources(client).await?;
  let nodes = resources::get_nodes(client).await?;

  let version_skew = checks::version_skew(&nodes, cluster_version)?;
  let min_replicas: Vec<checks::MinReplicas> = resources.iter().filter_map(|s| s.min_replicas()).collect();
  let min_ready_seconds: Vec<checks::MinReadySeconds> =
    resources.iter().filter_map(|s| s.min_ready_seconds()).collect();
  let pod_topology_distribution: Vec<checks::PodTopologyDistribution> =
    resources.iter().filter_map(|s| s.pod_topology_distribution()).collect();
  let readiness_probe: Vec<checks::Probe> = resources.iter().filter_map(|s| s.readiness_probe()).collect();
  let termination_grace_period: Vec<checks::TerminationGracePeriod> =
    resources.iter().filter_map(|s| s.termination_grace_period()).collect();
  let docker_socket: Vec<checks::DockerSocket> = resources
    .iter()
    .filter_map(|s| s.docker_socket(target_version).ok().flatten())
    .collect();
  let kube_proxy_version_skew = checks::kube_proxy_version_skew(&resources, cluster_version)?;

  Ok(KubernetesFindings {
    version_skew,
    min_replicas,
    min_ready_seconds,
    readiness_probe,
    pod_topology_distribution,
    termination_grace_period,
    docker_socket,
    kube_proxy_version_skew,
  })
}
