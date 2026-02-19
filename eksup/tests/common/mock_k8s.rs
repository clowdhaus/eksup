use anyhow::{Result, bail};
use k8s_openapi::api::core::v1::ConfigMap;

use eksup::clients::K8sClients;
use eksup::k8s::resources::{ENIConfig, Node, StdResource};

/// Mock K8s client for testing
#[derive(Clone, Default)]
pub struct MockK8sClients {
  pub nodes: Vec<Node>,
  pub configmap: Option<ConfigMap>,
  pub eniconfigs: Vec<ENIConfig>,
  pub resources: Vec<StdResource>,
}

impl K8sClients for MockK8sClients {
  async fn get_nodes(&self) -> Result<Vec<Node>> {
    Ok(self.nodes.clone())
  }

  async fn get_configmap(&self, _namespace: &str, _name: &str) -> Result<Option<ConfigMap>> {
    Ok(self.configmap.clone())
  }

  async fn get_eniconfigs(&self) -> Result<Vec<ENIConfig>> {
    Ok(self.eniconfigs.clone())
  }

  async fn get_resources(&self) -> Result<Vec<StdResource>> {
    Ok(self.resources.clone())
  }
}

/// Mock that returns errors for all methods
pub struct MockK8sClientsError;

impl K8sClients for MockK8sClientsError {
  async fn get_nodes(&self) -> Result<Vec<Node>> { bail!("mock K8s error") }
  async fn get_configmap(&self, _namespace: &str, _name: &str) -> Result<Option<ConfigMap>> { bail!("mock K8s error") }
  async fn get_eniconfigs(&self) -> Result<Vec<ENIConfig>> { bail!("mock K8s error") }
  async fn get_resources(&self) -> Result<Vec<StdResource>> { bail!("mock K8s error") }
}
