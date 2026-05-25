use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::warn;

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
  pub pod_disruption_budgets: Vec<checks::MissingPdb>,
}

/// Findings that were dropped by the ignore filter — same shape as
/// `KubernetesFindings`, minus the cluster-level checks (version_skew,
/// kube_proxy_version_skew, kube_proxy_ipvs_mode) which the filter never
/// touches. The wrapper struct always serializes (no `skip_serializing_if`
/// anywhere) so the JSON `suppressed:` key is always present.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct KubernetesSuppressed {
  pub min_replicas: Vec<checks::MinReplicas>,
  pub min_ready_seconds: Vec<checks::MinReadySeconds>,
  pub readiness_probe: Vec<checks::Probe>,
  pub pod_topology_distribution: Vec<checks::PodTopologyDistribution>,
  pub termination_grace_period: Vec<checks::TerminationGracePeriod>,
  pub docker_socket: Vec<checks::DockerSocket>,
  pub ingress_nginx_retirement: Vec<checks::IngressNginxRetirement>,
  pub pod_disruption_budgets: Vec<checks::MissingPdb>,
}

impl KubernetesSuppressed {
  pub fn total(&self) -> usize {
    self.min_replicas.len()
      + self.min_ready_seconds.len()
      + self.readiness_probe.len()
      + self.pod_topology_distribution.len()
      + self.termination_grace_period.len()
      + self.docker_socket.len()
      + self.ingress_nginx_retirement.len()
      + self.pod_disruption_budgets.len()
  }
}

pub async fn get_kubernetes_findings(
  k8s: &impl K8sClients,
  control_plane_minor: i32,
  target_minor: i32,
  checks_config: &crate::config::ChecksConfig,
) -> Result<(KubernetesFindings, KubernetesSuppressed)> {
  use crate::k8s::filter::apply_ignores;

  let resources = k8s.get_resources().await?;
  let nodes = k8s.get_nodes().await?;
  let kube_proxy_config = k8s.get_configmap("kube-system", "kube-proxy-config").await?;
  let pdbs = k8s.get_pod_disruption_budgets().await?;

  let version_skew = checks::version_skew(&nodes, control_plane_minor);

  // K8S002: still uses the existing s.min_replicas(&K8s002Config) which
  // pre-filters by ignore. Task 6 refactors this to threshold_for + no ignore.
  // For this commit, ignored K8S002 resources still vanish at construction; they
  // appear in KubernetesSuppressed only after Task 6 lands.
  let min_replicas: Vec<checks::MinReplicas> = resources
    .iter()
    .filter_map(|s| s.min_replicas(&checks_config.k8s002))
    .collect();

  let min_ready_seconds: Vec<checks::MinReadySeconds> =
    resources.iter().filter_map(|s| s.min_ready_seconds()).collect();
  let pod_topology_distribution: Vec<checks::PodTopologyDistribution> =
    resources.iter().filter_map(|s| s.pod_topology_distribution()).collect();
  let readiness_probe: Vec<checks::Probe> = resources.iter().filter_map(|s| s.readiness_probe()).collect();
  let termination_grace_period: Vec<checks::TerminationGracePeriod> =
    resources.iter().filter_map(|s| s.termination_grace_period()).collect();
  let docker_socket: Vec<checks::DockerSocket> = resources
    .iter()
    .filter_map(|s| match s.docker_socket() {
      Ok(finding) => finding,
      Err(e) => {
        warn!(
          "Failed to check docker socket for {}/{}: {e}",
          s.metadata.namespace, s.metadata.name
        );
        None
      }
    })
    .collect();
  let kube_proxy_version_skew = checks::kube_proxy_version_skew(&resources, control_plane_minor)?;
  let kube_proxy_ipvs_mode = checks::kube_proxy_ipvs_mode(kube_proxy_config.as_ref(), target_minor)?;
  let ingress_nginx_retirement = checks::ingress_nginx_retirement(&resources, target_minor)?;
  let pod_disruption_budgets = checks::pod_disruption_budgets(&resources, &pdbs);

  // Apply ignores to the 8 workload-level finding Vecs. Cluster-level Vecs
  // (version_skew, kube_proxy_version_skew, kube_proxy_ipvs_mode) skip the
  // filter — their finding structs don't impl WorkloadFinding.
  let compiled = checks_config.compiled()?;
  let (min_replicas, sup_min_replicas) = apply_ignores(min_replicas, compiled);
  let (min_ready_seconds, sup_min_ready_seconds) = apply_ignores(min_ready_seconds, compiled);
  let (readiness_probe, sup_readiness_probe) = apply_ignores(readiness_probe, compiled);
  let (pod_topology_distribution, sup_pod_topology_distribution) =
    apply_ignores(pod_topology_distribution, compiled);
  let (termination_grace_period, sup_termination_grace_period) = apply_ignores(termination_grace_period, compiled);
  let (docker_socket, sup_docker_socket) = apply_ignores(docker_socket, compiled);
  let (ingress_nginx_retirement, sup_ingress_nginx_retirement) = apply_ignores(ingress_nginx_retirement, compiled);
  let (pod_disruption_budgets, sup_pod_disruption_budgets) = apply_ignores(pod_disruption_budgets, compiled);

  Ok((
    KubernetesFindings {
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
      pod_disruption_budgets,
    },
    KubernetesSuppressed {
      min_replicas: sup_min_replicas,
      min_ready_seconds: sup_min_ready_seconds,
      readiness_probe: sup_readiness_probe,
      pod_topology_distribution: sup_pod_topology_distribution,
      termination_grace_period: sup_termination_grace_period,
      docker_socket: sup_docker_socket,
      ingress_nginx_retirement: sup_ingress_nginx_retirement,
      pod_disruption_budgets: sup_pod_disruption_budgets,
    },
  ))
}
