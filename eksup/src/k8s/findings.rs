use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
  clients::K8sClients,
  k8s::checks::{self, K8sFindings},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct KubernetesFindings {
  pub version_skew: Vec<checks::VersionSkew>,
  pub min_replicas: Vec<checks::MinReplicas>,
  pub min_ready_seconds: Vec<checks::MinReadySeconds>,
  pub readiness_probe: Vec<checks::Probe>,
  pub pod_topology_distribution: Vec<checks::PodTopologyDistribution>,
  pub termination_grace_period: Vec<checks::TerminationGracePeriod>,
  pub docker_socket: Vec<checks::DockerSocket>,
  pub kube_proxy_version_skew: Vec<checks::KubeProxyVersionSkew>,
  pub kube_proxy_ipvs_mode: Vec<checks::KubeProxyIpvsMode>,
  pub ingress_nginx_retirement: Vec<checks::IngressNginxRetirement>,
}

pub async fn get_kubernetes_findings(
  k8s: &impl K8sClients,
  control_plane_minor: i32,
  target_minor: i32,
) -> Result<KubernetesFindings> {
  let resources = k8s.get_resources().await?;
  let nodes = k8s.get_nodes().await?;
  let kube_proxy_config = k8s.get_configmap("kube-system", "kube-proxy-config").await?;

  let version_skew = checks::version_skew(&nodes, control_plane_minor);
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
    .filter_map(|s| s.docker_socket().ok().flatten())
    .collect();
  let kube_proxy_version_skew = checks::kube_proxy_version_skew(&resources, control_plane_minor)?;
  let kube_proxy_ipvs_mode = checks::kube_proxy_ipvs_mode(kube_proxy_config.as_ref(), target_minor)?;
  let ingress_nginx_retirement = checks::ingress_nginx_retirement(&resources, target_minor)?;

  Ok(KubernetesFindings {
    version_skew,
    min_replicas,
    min_ready_seconds,
    readiness_probe,
    pod_topology_distribution,
    termination_grace_period,
    docker_socket,
    kube_proxy_version_skew,
    kube_proxy_ipvs_mode,
    ingress_nginx_retirement,
  })
}
