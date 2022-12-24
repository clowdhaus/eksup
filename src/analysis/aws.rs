use aws_sdk_eks::{model::Cluster, Client};

pub async fn describe_cluster(client: &Client, name: &str) -> Result<Cluster, anyhow::Error> {
    let req = client.describe_cluster().name(name);
    let resp = req.send().await?;

    println!("{:#?}", resp.cluster);
    Ok(resp.cluster.unwrap())
}
