use k8s_openapi::api::core::v1::{Node, NodeSystemInfo};
use kube::{api::Api, Client};

/// Returns all of the nodes in the cluster with their information
///
/// This is similar to running `kubectl get nodes -o wide`
/// but containing the system info of the instance itself
pub async fn get_nodes(client: &Client) -> Result<Vec<NodeSystemInfo>, anyhow::Error> {
  let api_nodes: Api<Node> = Api::all(client.clone());
  let node_list = api_nodes.list(&Default::default()).await?;

  let nodes = node_list
    .items
    .iter()
    .map(|node| {
      node
        .status
        .as_ref()
        .unwrap()
        .node_info
        .as_ref()
        .unwrap()
        .clone()
    })
    .collect();

  Ok(nodes)
}
