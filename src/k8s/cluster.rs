use k8s_openapi::api::core::v1::Node;
use kube::{api::Api, Client};

/// Returns all of the nodes in the cluster with their information
pub async fn get_nodes(client: &Client) -> Result<Vec<Node>, anyhow::Error> {
  let api_nodes: Api<Node> = Api::all(client.clone());
  let nodes = api_nodes.list(&Default::default()).await?;

  Ok(nodes.items)
}
