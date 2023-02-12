use anyhow::Result;
use kube::Client as K8sClient;
use serde::{Deserialize, Serialize};

use crate::{
  finding::Findings,
  k8s::{
    checks::{self, K8sFindings},
    resources,
  },
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KubernetesFindings {
  pub min_replicas: Option<String>,
  pub min_ready_seconds: Option<String>,
}

pub async fn get_kubernetes_findings(k8s_client: &K8sClient) -> Result<KubernetesFindings> {
  let resources = resources::get_resources(k8s_client).await?;

  let min_replicas: Vec<checks::MinReplicas> = resources.iter().filter_map(|s| s.min_replicas()).collect();
  let min_ready_seconds: Vec<checks::MinReadySeconds> =
    resources.iter().filter_map(|s| s.min_ready_seconds()).collect();

  Ok(KubernetesFindings {
    min_replicas: min_replicas.to_markdown_table("\t"),
    min_ready_seconds: min_ready_seconds.to_markdown_table("\t"),
  })
}
