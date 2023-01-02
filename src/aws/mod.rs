use aws_sdk_eks::{model::Cluster, Client};
use serde::{Deserialize, Serialize};

use super::cli::Analysis;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EksControlPlane {
  subnet_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EksCluster {
  name: String,

  control_plane: EksControlPlane,
}

impl EksCluster {
  pub async fn get(client: &Client, analysis: &Analysis) -> Result<Cluster, anyhow::Error> {
    let req = client.describe_cluster().name(&analysis.cluster_name);
    let resp = req.send().await?;

    // TODO - handle error check here for cluster not found
    let cluster = &resp
      .cluster
      .expect(&format!("Cluster {} not found", &analysis.cluster_name));
    let subnet_ids = &cluster.resources_vpc_config.unwrap().subnet_ids.unwrap();

    let subnets = cluster
      .clone()
      .resources_vpc_config
      .unwrap()
      .subnet_ids
      .unwrap();

    println!("{:#?}", cluster);
    Ok(cluster)
  }
}
